use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::rmt::{PinState, Pulse, PulseTicks, Symbol, TxChannelDriver};
use esp_idf_svc::hal::rmt::config::TransmitConfig;
use esp_idf_svc::hal::rmt::encoder::CopyEncoder;
use anyhow::Result;

// IR Signal der Fernbedienung
const IR_DATA_BEAMER: u32 = 0xC12FE817;

pub struct Beamer<'a> {
    // Wir speichern nur den fertigen Treiber
    tx_driver: TxChannelDriver<'a>,
}

impl<'a> Beamer<'a> {
    // Konstruktor nimmt fertigen Treiber entgegen:
    pub fn new(tx_driver: TxChannelDriver<'a>) -> Self {
        Self { tx_driver }
    }

    // Beamer einschalten:
    pub fn turn_on(&mut self) -> Result<()> {
        self.send_nec(IR_DATA_BEAMER)
    }

    // Beamer ausschalten:
    pub fn turn_off(&mut self) -> Result<()> {
        self.send_nec(IR_DATA_BEAMER)?;
        FreeRtos::delay_ms(2000);
        self.send_nec(IR_DATA_BEAMER)?;
        Ok(())
    }

    // Hilfsfunktion senden der Signale
    fn send_nec(&mut self, command: u32) -> Result<()> {
        let mut symbols = Vec::new();

        symbols.push(Symbol::new(
            Pulse::new(PinState::High, PulseTicks::new(9000)?),
            Pulse::new(PinState::Low, PulseTicks::new(4500)?),
        ));

        for i in 0..32 {
            let bit = (command >> i) & 1;
            let mark = Pulse::new(PinState::High, PulseTicks::new(560)?);
            let space = Pulse::new(
                PinState::Low,
                if bit == 1 { PulseTicks::new(1690)? } else { PulseTicks::new(560)? },
            );
            symbols.push(Symbol::new(mark, space));
        }

        symbols.push(Symbol::new(
            Pulse::new(PinState::High, PulseTicks::new(560)?),
            Pulse::new(PinState::Low, PulseTicks::new(0)?),
        ));

        // 1. Einen CopyEncoder erstellen
        let mut encoder = CopyEncoder::new()?;

        // 2. Standard-Übertragungseinstellungen (kein Loop, etc.)
        let tx_config = TransmitConfig::default();

        // 3. Die drei Parameter übergeben
        self.tx_driver.send_and_wait(&mut encoder, &symbols, &tx_config)?;

        Ok(())
    }
}