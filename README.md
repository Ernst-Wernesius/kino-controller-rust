# kino-controller-rust

Ein MQTT-IoT-Gerät auf Basis eines ESP32, das reine Infrarot-Geräte "smart" macht.
Der ESP32 empfängt Befehle über MQTT und setzt sie in IR-Signale um. So lassen sich
Geräte ohne eigene Netzwerkschnittstelle, hier ein Beamer und eine motorisierte
Leinwand, in ein Smart Home wie Home Assistant einbinden. Zusätzlich meldet er
Tastendrücke einer IR-Fernbedienung per MQTT weiter.

Geschrieben in Rust mit [`esp-idf-svc`](https://github.com/esp-rs/esp-idf-svc).

## Funktionen

- **IR-Sender** für Beamer (NEC-Protokoll) und Leinwand (Rohdaten)
- **IR-Empfänger**, der eine bestimmte Fernbedienungstaste erkennt und per MQTT meldet
- **MQTT** mit Online-Status und Last Will (`online` / `offline`)
- **Sperrzeiten** pro Gerät, damit Befehle nicht zu schnell hintereinander gesendet werden
- **Automatisches Neuverbinden** bei WLAN-Ausfall, Neustart bei Fehlern während des Starts
- **Logging per UDP** an einen PC im Netzwerk

## Hardware

| Komponente | Pin |
|---|---|
| IR-Empfänger (38 kHz) | GPIO15 |
| IR-LED Leinwand | GPIO5 |
| IR-LED Beamer | GPIO4 |

Getestet mit einem ESP32 (Xtensa). Die IR-Sender arbeiten mit 38 kHz Trägerfrequenz.

## MQTT-Topics

| Topic | Richtung | Payload | Hinweis |
|---|---|---|---|
| `cinema/esp/status` | ESP → Broker | `online` / `offline` | retained, `offline` als Last Will |
| `cinema/beamer/set` | → ESP | `ON` / `OFF` | |
| `cinema/screen/set` | → ESP | `UP` / `DOWN` | |
| `cinema/rii/button` | ESP → | `SET` | nicht retained |

Befehle sollten **nicht retained** gesendet werden, sonst werden sie bei jeder
Neuverbindung erneut ausgeführt.

### Sperrzeiten

IR ist eine Einbahnstraße, der ESP kennt den Zustand der Geräte nicht. Um doppelte
Befehle zu vermeiden, ignoriert er für eine gewisse Zeit weitere Befehle an dasselbe Gerät:

| Gerät | Sperre |
|---|---|
| Beamer | 30 s nach Einschalten, 5 s nach Ausschalten |
| Leinwand | 15 s nach jeder Aktion |

## Aufbau

```
IR-Thread    ──┐
MQTT-Thread  ──┼──► Kanal (mpsc) ──► Haupt-Thread: Geräte steuern, MQTT senden
WLAN-Ereignis──┘
```

Die Threads erzeugen nur Ereignisse (`Event`), der Haupt-Thread besitzt alle Geräte
und führt die Aktionen aus. Dadurch braucht das Programm keine Mutexe.

| Datei | Inhalt |
|---|---|
| `main.rs` | Initialisierung und Ereignis-Schleife |
| `event.rs` | Ereignistypen für den Kanal |
| `mqtt_client.rs` | Topics und MQTT-Listener-Thread |
| `ir_receiver.rs` | IR-Empfang und NEC-Dekodierung |
| `beamer.rs` | IR-Steuerung des Beamers |
| `screen.rs` | IR-Steuerung der Leinwand |
| `wifi.rs` | WLAN-Verbindung |

## Konfiguration

Die Zugangsdaten werden beim Kompilieren aus einer `.env`-Datei im Projektordner
gelesen. Sie ist in der `.gitignore` und wird nicht eingecheckt.

```
SSID=dein-wlan
PASSWORD=dein-wlan-passwort
PC_IP=192.168.8.20
MQTT_URL=mqtt://mqtt.lan:1883
MQTT_USER=esp-kino
MQTT_PASS=dein-mqtt-passwort
MQTT_CLIENT=esp-kino
```

Nach Änderungen an der `.env` muss neu kompiliert werden.

## Bauen und Flashen

Voraussetzung ist die ESP-Rust-Toolchain, eingerichtet mit
[`espup`](https://github.com/esp-rs/espup), sowie
[`espflash`](https://github.com/esp-rs/espflash).

```bash
source ~/export-esp.sh
cargo run --release
```

`cargo run` baut das Projekt, flasht es auf den ESP32 und öffnet den seriellen Monitor.

### Logs empfangen

Der ESP sendet Log-Meldungen per UDP an `PC_IP` auf Port 4211:

```bash
nc -klu 4211
```

## Home Assistant

Ein Beispiel für die Einbindung als MQTT-Buttons in der `configuration.yaml`:

```yaml
mqtt:
  button:
    - name: "Beamer an"
      unique_id: kino_beamer_an
      command_topic: "cinema/beamer/set"
      payload_press: "ON"
      retain: false
      availability_topic: "cinema/esp/status"
```

Die Rii-Taste lässt sich in einer Automation mit einem MQTT-Auslöser auf
`cinema/rii/button` verwenden.
