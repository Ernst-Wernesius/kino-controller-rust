use std::sync::mpsc::Sender;

use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::InputPin;
use esp_idf_svc::hal::rmt::config::{ReceiveConfig, RxChannelConfig};
use esp_idf_svc::hal::rmt::{PinState, Pulse, PulseTicks, RxChannelDriver, Symbol};
use esp_idf_svc::hal::units::Hertz;

use crate::event::Event;

// IR_RII_REMOTE:
const IR_REMOTE_ADDR: u32 = 0x560A;
const IR_REMOTE_CMD: u32 = 0x88;

// Debug Logging Konstante:
const LOG_RAW_SIGNALS: bool = false;

#[derive(Debug)]
pub enum IrStatus {
    Valid(u32, u32),
    Raw(Vec<u16>),
    Nothing,
}

pub struct IrReceiver<'a> {
    rx_driver: RxChannelDriver<'a>,
}

impl<'a> IrReceiver<'a> {
    pub fn new(rx_driver: RxChannelDriver<'a>) -> Self {
        Self { rx_driver }
    }

    pub fn listen(&mut self) -> IrStatus {
        let empty_pulse = Pulse::new(PinState::Low, PulseTicks::new(1).unwrap());
        let empty_symbol = Symbol::new(empty_pulse, empty_pulse);
        let mut symbols = [empty_symbol; 100];

        let rx_config = ReceiveConfig {
            signal_range_max: std::time::Duration::from_millis(50),
            ..Default::default()
        };

        if let Ok(rx_size) = self.rx_driver.receive(&mut symbols, &rx_config) {
            if rx_size > 0 {
                // 1. Rohdaten extrahieren (Mark und Space in µs)
                let mut raw_timings: Vec<u16> = Vec::with_capacity(rx_size * 2);
                for i in 0..rx_size {
                    let raw: u32 = unsafe { std::mem::transmute(symbols[i]) };
                    let mark = (raw & 0x7FFF) as u16;
                    let space = ((raw >> 16) & 0x7FFF) as u16;
                    raw_timings.push(mark);
                    raw_timings.push(space);
                }
                // Ein gültiger NEC-Frame hat mindestens Start-Bit + 32 Datenbits = 33 Symbole
                if rx_size >= 33 {
                    let mut raw_bits: u32 = 0;
                    let mut decoded_bits = 0;

                    // Symbol 0 ist der Leading Pulse (9ms + 4.5ms) -> überspringen
                    // Symbole 1 bis 32 sind die Datenbits
                    for i in 1..rx_size {
                        if decoded_bits >= 32 {
                            break;
                        }

                        let raw: u32 = unsafe { std::mem::transmute(symbols[i]) };
                        // Dauer der High-Phase (Pulse 2, Bits 16..30)
                        let high_duration_us = (raw >> 16) & 0x7FFF;

                        // Bit ermitteln: ~560µs = 0, ~1690µs = 1 (Schwellwert bei 1100µs)
                        if high_duration_us > 1100 {
                            raw_bits |= 1 << decoded_bits;
                        }
                        decoded_bits += 1;
                    }

                    if decoded_bits == 32 {
                        // NEC Extended Format:
                        // Bits 0..15: 16-Bit Adresse (LSB first)
                        // Bits 16..23: 8-Bit Command
                        // Bits 24..31: 8-Bit Inverted Command
                        let addr = raw_bits & 0xFFFF;
                        let cmd = (raw_bits >> 16) & 0xFF;
                        let cmd_inv = (raw_bits >> 24) & 0xFF;

                        // Optionale Prüfsummenkontrolle (cmd == !cmd_inv)
                        if (cmd ^ cmd_inv) == 0xFF {
                            return IrStatus::Valid(addr, cmd);
                        }
                    }
                }
                return IrStatus::Raw(raw_timings);
            }
        }
        IrStatus::Nothing
    }
}

pub fn spawn_ir_listener(
    pin: impl InputPin + Send + 'static,
    tx: Sender<Event>,
) -> anyhow::Result<()> {
    std::thread::Builder::new()
        .stack_size(8 * 1024)
        .spawn(move || {
            // Config für IR Empfänger:
            let rx_config = RxChannelConfig {
                resolution: Hertz(1_000_000), // 1 Tick = 1 Mikrosekunde
                ..Default::default()
            };
            // Hardware-Treiber:
            let rx_driver = match RxChannelDriver::new(pin, &rx_config) {
                Ok(d) => d,
                Err(e) => {
                    let _ = tx.send(Event::Log(format!("IR-Treiber Fehler: {:?}", e)));
                    return;
                }
            };
            // An IR Receiver übergeben:
            let mut receiver = IrReceiver::new(rx_driver);

            loop {
                match receiver.listen() {
                    // FALL 1: Ein sauberer NEC-Befehl wurde decodiert!
                    IrStatus::Valid(addr, cmd) => {
                        if addr == IR_REMOTE_ADDR && cmd == IR_REMOTE_CMD {
                            let _ = tx.send(Event::RiiKey);
                            FreeRtos::delay_ms(500);
                        } else if addr != 0 || cmd != 0 {
                            let _ = tx.send(Event::Log(format!(
                                "Unbekanntes NEC Signal - Adresse: {:#X}, Befehl: {:#X}",
                                addr, cmd
                            )));
                        }
                    }

                    // FALL 2: Es kam Licht an, aber der Decoder konnte es nicht lesen
                    IrStatus::Raw(timings) => {
                        if LOG_RAW_SIGNALS {
                            let _ = tx.send(Event::Log(format!(
                                "Raw-Signal ({} Werte): {:?}",
                                timings.len(),
                                timings
                            )));
                        }
                    }

                    // FALL 3: Es kam einfach nichts an (Timeout)
                    IrStatus::Nothing => {
                        // Hier tun wir absolut gar nichts und die Schleife startet leise von vorn
                    }
                }
            }
        })?;
    Ok(())
}
