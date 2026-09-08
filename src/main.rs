use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::{EspDefaultNvsPartition};
use std::net::UdpSocket;
use anyhow::Result;
use esp_idf_svc::hal::gpio::{PinDriver, Pull};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::wifi::EspWifi;
use dotenvy_macro::dotenv;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::rmt::{RmtChannel, TxChannelDriver};
use esp_idf_svc::hal::units::Hertz;
use esp_idf_svc::wifi::BlockingWifi;
use esp_idf_svc::hal::rmt::config::{TxChannelConfig, CarrierConfig, DutyPercent};

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
const PC_MAC: [u8;6] = mac_parse::PC_MAC ;

// IR_RII_REMOTE:
const IR_DATA_REMOTE: u32 = 0x506A11EE;

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
    let pin_ir_receiver = peripherals.pins.gpio15;
    let pin_ir_screen = peripherals.pins.gpio5;
    let pin_beamer = peripherals.pins.gpio4;

    // Configs für IR-Sender:
    // RMT-Konfiguration:
    let rmt_config = TxChannelConfig {
        resolution: Hertz(1_000_000),
        ..Default::default()
    };
    // Carrier Konfiguration:
    let carrier_config = CarrierConfig {
        frequency: Hertz(38_000),
        duty_cycle: DutyPercent::new(50f32)?,
        ..Default::default()
    };

    // Structs:
    // IR_Receiver:
    let mut _ir_receiver = ir_receiver::IrReceiver::new(
        PinDriver::input(pin_ir_receiver, Pull::Floating)?
    );

    // Beamer:
    // Treiber erstellen:
    let mut beamer_driver = TxChannelDriver::new(
        pin_beamer,
        &rmt_config,
    )?;
    // Carrier hinzufügen:
    beamer_driver.apply_carrier(Some(&carrier_config))?;
    // Treiber an Beamer übergeben:
    let mut _beamer = beamer::Beamer::new(beamer_driver);

    // Leinwand:
    // Treiber erstellen:
    let mut screen_driver = TxChannelDriver::new(
        pin_ir_screen,
        &rmt_config,
    )?;
    // Carrier hinzufügen:
    screen_driver.apply_carrier(Some(&carrier_config))?;
    // Treiber übergeben
    let mut _screen = screen::Screen::new(screen_driver);

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

                    start_cinema(&net_log, &mut _beamer, &mut _screen);

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
        FreeRtos::delay_ms(50);
    }
}

fn start_cinema(log: &impl Fn(&str), beamer: &mut beamer::Beamer, screen: &mut screen::Screen) {
    log("Starting cinema");

    // Beamer einschalten:
    log("Starting beamer");
    let _ = beamer.turn_on();

    log("Screen down");
    let _ = screen.move_down();
}
