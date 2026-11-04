use anyhow::Result;
use dotenvy_macro::dotenv;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::rmt::config::{CarrierConfig, DutyPercent, TxChannelConfig};
use esp_idf_svc::hal::rmt::{RmtChannel, TxChannelDriver};
use esp_idf_svc::hal::units::Hertz;
use esp_idf_svc::mqtt::client::{EspMqttClient, LwtConfiguration, MqttClientConfiguration, QoS};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::EspWifi;
use esp_idf_svc::wifi::{BlockingWifi, WifiEvent};
use std::net::UdpSocket;
use std::sync::mpsc;
use std::time::Duration;

use crate::event::{BeamerCommand, Event, ScreenDirection};

pub mod beamer;
pub mod event;
pub mod ir_receiver;
pub mod mqtt_client;
pub mod screen;
pub mod wifi;

// Konstanten definieren:
// WLAN:
const SSID: &str = dotenv!("SSID");
const PASSWORD: &str = dotenv!("PASSWORD");

// PC IP-Adresse fürs Logging:
const PC_IP: &str = dotenv!("PC_IP");
const DEBUG_PORT: u16 = 4211;

// MQTT:
const MQTT_URL: &str = dotenv!("MQTT_URL");
const MQTT_USER: &str = dotenv!("MQTT_USER");
const MQTT_PASSWD: &str = dotenv!("MQTT_PASS");
const MQTT_CLIENT: &str = dotenv!("MQTT_CLIENT");

fn main() {
    // Patches und Logger:
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    if let Err(e) = run() {
        log::error!("Error: {:?}", e);
    }

    log::error!("Run has endet, restarting in 5 seconds");
    FreeRtos::delay_ms(5000);
    esp_idf_svc::hal::reset::restart();
}

fn run() -> Result<()> {
    // Event Channel aufbauen
    let (tx, rx) = mpsc::channel();

    // Hardware holen:
    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // WLAN Treiber für die Hilfsfunktion bereit holen:
    let mut wifi_driver = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop.clone(),
    )?;

    // Wifi Subscription für die Überwachung der WLAN Verbindung:
    let wifi_tx = tx.clone();
    let _wifi_subscription = sys_loop.subscribe::<WifiEvent, _>(move |event| {
        if matches!(event, WifiEvent::StaDisconnected(..)) {
            let _ = wifi_tx.send(Event::WifiLost);
        }
    })?;

    // WLAN verbinden
    wifi::connect_wifi(&mut wifi_driver, SSID, PASSWORD)?;

    // UDP Socket einrichten zum Loggen:
    let udp_socket = UdpSocket::bind("0.0.0.0:0")?;
    let pc_address = format!("{}:{}", PC_IP, DEBUG_PORT);

    // Log Funktion bauen:
    let net_log = |msg: &str| {
        // String formatieren und mit Zeilenumbruch versehen
        let formatted = format!("{}\n", msg);
        // Übers Netzwerk an den PC schicken
        let _ = udp_socket.send_to(formatted.as_bytes(), &pc_address);
    };

    // Test-Log in den Kanal schmeißen:
    net_log("WLAN connected");

    // Pins definieren:
    let pin_ir_receiver = peripherals.pins.gpio15;
    let pin_screen = peripherals.pins.gpio5;
    let pin_beamer = peripherals.pins.gpio4;

    // Konfigurationen:
    // RMT-Konfiguration:
    let rmt_config = TxChannelConfig {
        resolution: Hertz(1_000_000),
        ..Default::default()
    };

    // Carrier Konfiguration:
    let carrier_config = CarrierConfig {
        frequency: Hertz(38_000),
        duty_cycle: DutyPercent::new(0.33)?,
        ..Default::default()
    };

    // Config für MQTT Client:
    let mqtt_config = MqttClientConfiguration {
        client_id: Some(MQTT_CLIENT),
        username: Some(MQTT_USER),
        password: Some(MQTT_PASSWD),
        lwt: Some(LwtConfiguration {
            topic: mqtt_client::TOPIC_STATUS,
            payload: b"offline",
            qos: QoS::AtLeastOnce,
            retain: true,
        }),
        keep_alive_interval: Some(Duration::from_secs(30)),
        ..Default::default()
    };

    // Structs:
    // Beamer:
    // Treiber erstellen:
    let mut beamer_driver = TxChannelDriver::new(pin_beamer, &rmt_config)?;
    // Carrier hinzufügen:
    beamer_driver.apply_carrier(Some(&carrier_config))?;
    // Treiber an Beamer übergeben:
    let mut _beamer = beamer::Beamer::new(beamer_driver);

    // Leinwand:
    // Treiber erstellen:
    let mut screen_driver = TxChannelDriver::new(pin_screen, &rmt_config)?;
    // Carrier hinzufügen:
    screen_driver.apply_carrier(Some(&carrier_config))?;
    // Treiber übergeben
    let mut _screen = screen::Screen::new(screen_driver);

    // MQTT Client:
    let (mut _mqtt_client, mqtt_connection) = EspMqttClient::new(MQTT_URL, &mqtt_config)?;

    mqtt_client::spawn_mqtt_listener(mqtt_connection, tx.clone())?;

    ir_receiver::spawn_ir_listener(pin_ir_receiver, tx.clone())?;

    // Verarbeitung der Events aus den beiden Threats
    for event in rx {
        match event {
            Event::Connected => {
                if let Err(e) = _mqtt_client.publish(
                    mqtt_client::TOPIC_STATUS,
                    QoS::AtLeastOnce,
                    true,
                    b"online",
                ) {
                    net_log(&format!("Publish failed: {:?}", e));
                }
                if let Err(e) =
                    _mqtt_client.subscribe(mqtt_client::TOPIC_BEAMER_SET, QoS::AtMostOnce)
                {
                    net_log(&format!("Subscribe topic failed: {:?}", e));
                }
                if let Err(e) =
                    _mqtt_client.subscribe(mqtt_client::TOPIC_SCREEN_SET, QoS::AtMostOnce)
                {
                    net_log(&format!("Subscribe topic failed: {:?}", e));
                }
            }
            Event::Beamer(BeamerCommand::On) => {
                if let Err(e) = _beamer.turn_on() {
                    net_log(&format!("Turning on Beamer failed: {:?}", e));
                }
            }
            Event::Beamer(BeamerCommand::Off) => {
                if let Err(e) = _beamer.turn_off() {
                    net_log(&format!("Turning off Beamer failed: {:?}", e));
                }
            }
            Event::Screen(ScreenDirection::Up) => {
                if let Err(e) = _screen.move_up() {
                    net_log(&format!("Pulling Screen up failed: {:?}", e));
                }
            }
            Event::Screen(ScreenDirection::Down) => {
                if let Err(e) = _screen.move_down() {
                    net_log(&format!("Pulling Screen down failed: {:?}", e));
                }
            }
            Event::RiiKey => {
                if let Err(e) =
                    _mqtt_client.publish(mqtt_client::TOPIC_RII, QoS::AtLeastOnce, false, b"SET")
                {
                    net_log(&format!("Publish failed: {:?}", e));
                }
            }
            Event::Log(msg) => net_log(&msg),
            Event::WifiLost => {
                if !wifi_driver.is_connected().unwrap_or(false) {
                    FreeRtos::delay_ms(2000);
                    if let Err(e) = wifi_driver.connect() {
                        log::error!("Error with the wifi connection: {:?}", e);
                    } else {
                        if let Err(e) = wifi_driver.wait_netif_up() {
                            log::error!("Error with the wifi connection: {:?}", e);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
