use anyhow::Result;
use dotenvy_macro::dotenv;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::rmt::config::RxChannelConfig;
use esp_idf_svc::hal::rmt::config::{CarrierConfig, DutyPercent, TxChannelConfig};
use esp_idf_svc::hal::rmt::RxChannelDriver;
use esp_idf_svc::hal::rmt::{RmtChannel, TxChannelDriver};
use esp_idf_svc::hal::units::Hertz;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::BlockingWifi;
use esp_idf_svc::wifi::EspWifi;
use std::net::UdpSocket;

pub mod beamer;
pub mod ir_receiver;
pub mod screen;
pub mod wifi;

// Konstanten definieren:
// WLAN:
const SSID: &str = dotenv!("SSID");
const PASSWORD: &str = dotenv!("PASSWORD");

// PC IP-Adresse fürs Logging:
const PC_IP: &str = dotenv!("PC_IP");
const DEBUG_PORT: u16 = 4211;

// IR_RII_REMOTE:
const IR_REMOTE_ADDR: u32 = 0x560A;
const IR_REMOTE_CMD: u32 = 0x88;

fn main() -> Result<()> {
    // Patches und Logger:
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // Hardware holen:
    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // WLAN Treiber für die Hilfsfunktion bereit holen:
    let mut wifi_driver = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

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
    let pin_ir_screen = peripherals.pins.gpio5;
    let pin_beamer = peripherals.pins.gpio4;

    // Config für IR Empfänger:
    let rx_config = RxChannelConfig {
        resolution: Hertz(1_000_000), // 1 Tick = 1 Mikrosekunde
        ..Default::default()
    };

    // Configs für IR-Sender:
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

    // Structs:
    // IR_Receiver:
    // Hardware-Treiber:
    let rx_driver = RxChannelDriver::new(pin_ir_receiver, &rx_config)?;
    // An IR Receiver übergeben:
    let mut _ir_receiver = ir_receiver::IrReceiver::new(rx_driver);

    // Beamer:
    // Treiber erstellen:
    let mut beamer_driver = TxChannelDriver::new(pin_beamer, &rmt_config)?;
    // Carrier hinzufügen:
    beamer_driver.apply_carrier(Some(&carrier_config))?;
    // Treiber an Beamer übergeben:
    let mut _beamer = beamer::Beamer::new(beamer_driver);

    // Leinwand:
    // Treiber erstellen:
    let mut screen_driver = TxChannelDriver::new(pin_ir_screen, &rmt_config)?;
    // Carrier hinzufügen:
    screen_driver.apply_carrier(Some(&carrier_config))?;
    // Treiber übergeben
    let mut _screen = screen::Screen::new(screen_driver);

    // Endlosschleife - Polling auf den IR Receiver:
    loop {
        net_log("Listening for IR-Signal");
        // Wir prüfen exakt, welchen Status der Empfänger meldet
        match _ir_receiver.listen() {
            // FALL 1: Ein sauberer NEC-Befehl wurde decodiert!
            ir_receiver::IrStatus::Valid(addr, cmd) => {
                if addr == IR_REMOTE_ADDR && cmd == IR_REMOTE_CMD {
                    net_log("Rii Button received");
                } else if addr != 0 || cmd != 0 {
                    let debug_msg = format!(
                        "Unbekanntes NEC Signal - Adresse: {:#X}, Befehl: {:#X}",
                        addr, cmd
                    );
                    net_log(&debug_msg);
                }
            }

            // FALL 2: Es kam Licht an, aber der Decoder konnte es nicht lesen
            ir_receiver::IrStatus::Raw(timings) => {
                let raw_str = format!("Raw-Signal ({} Werte): {:?}", timings.len(), timings);
                net_log(&raw_str);
            }

            // FALL 3: Es kam einfach nichts an (Timeout)
            ir_receiver::IrStatus::Nothing => {
                // Hier tun wir absolut gar nichts und die Schleife startet leise von vorn
            }
        }
    }
}
