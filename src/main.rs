use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition};
use std::sync::mpsc;
use std::net::UdpSocket;
use anyhow::Result;
use esp_idf_svc::hal::gpio::{PinDriver, Pull};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::wifi::EspWifi;
use dotenvy_macro::dotenv;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::rmt::config::TxChannelConfig;
use esp_idf_svc::hal::rmt::TxChannelDriver;
use esp_idf_svc::hal::units::Hertz;

pub mod receiver;
pub mod wifi;
pub mod ir_receiver;
pub mod mac_parse;
pub mod beamer;
pub mod screen;

// Konstanten definieren:
// WLAN:
const SSID: &str = dotenv!("SSID");
const PASSWORD: &str = dotenv!("PASSWORD");

// IP-Adressen:
const PC_IP: &str = dotenv!("PC_IP");
const DEBUG_PORT: u16 = 4211;

const RECEIVER_IP: &str = dotenv!("REC_IP");
const RECEIVER_PORT: u16 = 8102;

// MAC-Adresse für WakeonLan:
const PC_MAC = mac_parse::PC_MAC ;

// IR_RII_REMOTE:
const IR_DATA_REMOTE: u32 = 0x506A11EE;

fn main() -> Result<()> {
    // Patches und Logger:
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // Hardware holen:
    let peripherals = Peripherals::take().unwrap();
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // WLAN Treiber für die Hilfsfunktion bereit holen:
    let mut wifi_driver = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    // WLAN verbinden
    wifi::connect_wifi(&mut wifi_driver, SSID, PASSWORD);

    // UDP Socket einrichten zum loggen:
    let udp_socket = UdpSocket::bind("0.0.0.0:0")?;
    let pc_address = format!("{}:{}", PC_IP, DEBUG_PORT);

    // Log Funktion bauen:
    let net_log = |msg: &str| {
        // String formatieren und mit Zeilenumbruch versehen
        let formatted = format!("{}\n", msg);
        // Übers Netzwerk an den PC schicken
        let _ = udp_socket.send_to(formatted.as_bytes(), &pc_address);
    };

    // Test Log in den Kanal schmeißen:
    net_log("WLAN connected");

    // Pins definieren:
    let pin_ir_screen = peripherals.pins.gpio5;

    // Structs:
    // IR_Receiver:
    let mut _ir_receiver = ir_receiver::IrReceiver::new(
        PinDriver::input(peripherals.pins.gpio15, Pull::Floating)?
    );

    // Beamer:
    // RMT-Konfiguration:
    let beamer_rmt_config = TxChannelConfig {
        resolution: Hertz(1_000_000),
        carrier_enable: true,
        carrier_freq: Hertz(38_000),
        carrier_duty_percent: 50,
        ..Default::default()
    };
    // Treiber mit Kanal und Pin:
    let beamer_driver = TxChannelDriver::new(
        peripherals.rmt.channel0,
        peripherals.pins.gpio4,
        &beamer_rmt_config,
    )?;
    // Treiber an Beamer übergeben:
    let mut _beamer = beamer::Beamer::new(beamer_driver)?;

    // Leinwand:

    // Receiver:


    // Heimkino Variable:
    let mut cinema_is_running: bool = false;

    // Endlosschleife - Polling auf den IR Receiver:
    loop {
        // Signal entpacken:
        if let Some(command) = _ir_receiver.listen() {
            // Prüfen, ob das Signal übereinstimmt:
            if command == IR_DATA_REMOTE {
                net_log("Rii Button received");

                if !cinema_is_running {
                    net_log("Starting Cinema");

                    start_cinema(&mut _beamer);

                    // Status ändern:
                    cinema_is_running = true;

                    net_log("Cinema is running");
                } else {
                    net_log("Cinema is already running");
                }
            } else {
                // Vielleicht Mal füŕs Debugging neue Nachrichten
            }
        }
    }
    Ok(())
}

fn start_cinema(beamer: &mut beamer::Beamer,) {
    // Beamer einschalten:
    beamer.turn_on();
}
