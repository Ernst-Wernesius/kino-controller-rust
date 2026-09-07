use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition};
use std::sync::mpsc;
use std::net::UdpSocket;
use anyhow::Result;
use esp_idf_svc::hal::gpio::{PinDriver, Pull};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::wifi::EspWifi;

pub mod receiver;
pub mod wifi;
pub mod ir_receiver;

// Konstanten definieren:
// WLAN:
const SSID: &str = "**SSID**";
const PASSWORD: &str = "***WLAN_PASSWORD***";

// IP-Adressen:
const PC_IP: &str = "***PC_IP***";
const DEBUG_PORT: u16 = 4211;

const RECEIVER_IP: &str = "***REC_IP***";
const RECEIVER_PORT: u16 = 8102;

// MAC-Adresse für WakeonLan:
const PC_MAC: [u8; 6] = [***MAC-Adresse***];

// Infrarot Daten:
const IR_DATA_SCREEN_DOWN: [u16; 199] = [
    510, 1272, 1358, 428, 512, 1274, 1356, 430, 516, 1272,
    1410, 376, 514, 1272, 1358, 428, 514, 1272, 1358, 428,
    514, 1272, 1358, 428, 512, 1274, 1356, 430, 512, 1274,
    1408, 376, 512, 1272, 514, 1270, 512, 1272, 516, 1270,
    514, 1274, 1356, 428, 512, 1272, 516, 1268, 510, 13312,
    514, 1272, 1358, 428, 514, 1272, 1358, 428, 512, 1272,
    1406, 380, 512, 1272, 1354, 430, 514, 1272, 1354, 430,
    516, 1270, 1356, 430, 510, 1274, 1356, 428, 512, 1272,
    1358, 428, 512, 1272, 512, 1274, 512, 1272, 512, 1272,
    516, 1270, 1354, 430, 514, 1270, 512, 1268, 514, 13308,
    514, 1272, 1358, 428, 514, 1272, 1358, 428, 514, 1272,
    1358, 428, 512, 1272, 1406, 380, 512, 1272, 1354, 430,
    514, 1270, 1356, 430, 512, 1274, 1356, 430, 510, 1274,
    1408, 376, 512, 1274, 514, 1270, 512, 1272, 514, 1270,
    514, 1272, 1358, 428, 512, 1272, 512, 1270, 514, 13306,
    514, 1272, 1406, 378, 514, 1270, 1356, 430, 512, 1274,
    1406, 378, 510, 1272, 1358, 428, 512, 1272, 1354, 432,
    514, 1272, 1354, 430, 516, 1270, 1354, 430, 514, 1272,
    1354, 430, 516, 1270, 512, 1272, 516, 1270, 512, 1272,
    510, 1274, 1356, 430, 510, 1274, 514, 1268, 516
];
const IR_DATA_SCREEN_UP: [u16; 149] = [
    510, 1272, 1358, 428, 462, 1324, 1358, 428, 512, 1274,
    1356, 430, 464, 1322, 1360, 426, 516, 1272, 1358, 428,
    462, 1324, 1358, 428, 462, 1324, 1358, 428, 462, 1324,
    1358, 428, 462, 1324, 514, 1270, 512, 1272, 516, 1270,
    512, 1272, 516, 1270, 1360, 424, 464, 1318, 512, 13314,
    514, 1272, 1358, 428, 462, 1324, 1358, 428, 462, 1324,
    1358, 428, 462, 1324, 1358, 428, 460, 1324, 1358, 428,
    462, 1324, 1358, 428, 514, 1272, 1358, 428, 462, 1324,
    1358, 428, 462, 1324, 514, 1270, 514, 1272, 516, 1270,
    512, 1272, 516, 1270, 1356, 428, 464, 1318, 514, 13312,
    514, 1274, 1356, 430, 464, 1322, 1356, 430, 464, 1322,
    1356, 430, 464, 1322, 1356, 430, 464, 1322, 1356, 430,
    464, 1322, 1360, 426, 514, 1270, 1360, 426, 514, 1270,
    1356, 430, 464, 1322, 514, 1272, 516, 1270, 512, 1272,
    514, 1270, 512, 1272, 1358, 428, 462, 1320, 514
];
const IR_DATA_BEAMER: u32 = 0xC12FE817;
const IR_DATA_REMOTE: u32 = 0x506A11EE;

fn main() -> Result<()> {
    // Patches und Logger:
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // Hardware holen:
    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // WLAN verbinden
    let _wifi = wifi::connect_wifi(
        SSID,
        PASSWORD,
        peripherals.modem,
        sysloop,
        nvs
    )?;

    // MPSC Kanal und UDP Socket einrichten zum loggen:
    let (tx, rx) = mpsc::channel::<String>();
    let udp_socket = UdpSocket::bind("0.0.0.0:0")?;
    let pc_adresse = format!("{}:{}", PC_IP, DEBUG_PORT);

    // Test Log in den Kanal schmeißen:
    let _ = tx.send(String::from("WLAN connected"));

    // Pins definieren:
    let pin_ir_rec = PinDriver::input(peripherals.pins.gpio15, Pull::Floating)?;
    let pin_ir_beamer = peripherals.pins.gpio4;
    let pin_ir_screen = peripherals.pins.gpio5;

    // IR-Empfänger Modul instanziieren mit PIN:
    let mut ir_receiver_1 = ir_receiver::IrReceiver::new(pin_ir_rec);

    // Heimkino Variable:
    let mut cinema_is_running: bool = false;

    // Endlosschleife - Polling auf den IR Receiver:
    loop {
        let _ = tx.send(String::from("Waiting on IR-Signal"));
    }
}
