use esp_idf_svc::hal::rmt::{RxChannelDriver, Symbol, Pulse, PulseTicks, PinState};
use esp_idf_svc::hal::rmt::config::ReceiveConfig;

pub enum IrStatus {
    Valid(u32, u32),
    Raw(Vec<u16>),
    Nothing
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