use esp_idf_svc::hal::rmt::{PinState, Pulse, PulseTicks, Symbol, TxChannelDriver};
use esp_idf_svc::hal::rmt::config::TransmitConfig;
use esp_idf_svc::hal::rmt::encoder::CopyEncoder;
use anyhow::Result;
use esp_idf_svc::hal::delay::FreeRtos;

// Infrarot Daten:
const IR_DATA_SCREEN_DOWN: [u16; 50] = [
    460, 1330, 1400, 410, 480, 1300, 1330, 460, 440, 1350,
    1360, 430, 440, 1350, 1330, 460, 440, 1350, 1360, 430,
    440, 1350, 1380, 410, 440, 1350, 1360, 430, 440, 1350,
    1380, 410, 440, 1350, 480, 1300, 480, 1300, 430, 1350,
    440, 1350, 1380, 410, 480, 1300, 480, 1300, 430, 13370,
];
const IR_DATA_SCREEN_UP: [u16; 50] = [
    515, 1270, 1360, 450, 440, 1320, 1360, 450, 440, 1320,
    1360, 450, 440, 1320, 1360, 450, 440, 1320, 1360, 450,
    440, 1320, 1360, 450, 440, 1320, 1360, 450, 440, 1320,
    515, 1270, 515, 1270, 515, 1270, 515, 1270, 515, 1270,
    1360, 450, 440, 1320, 515, 1270, 515, 1270, 440, 13310,
];

pub struct Screen<'a> {
    tx_driver: TxChannelDriver<'a>
}

impl<'a> Screen<'a> {
    // Konstruktor:
    pub fn new(tx_driver: TxChannelDriver<'a>) -> Self {
        Self {tx_driver}
    }

    // Runterfahren:
    pub fn move_down(&mut self) -> Result<()> {
        for _ in 0..3 {
            self.send_raw(&IR_DATA_SCREEN_DOWN)?;
            FreeRtos::delay_ms(14);
        }
        Ok(())
    }

    // Hochfahren:
    pub fn move_up(&mut self) -> Result<()> {
        for _ in 0..3 {
            self.send_raw(&IR_DATA_SCREEN_UP)?;
            FreeRtos::delay_ms(14);
        }
        Ok(())
    }

    fn send_raw(&mut self, data: &[u16]) -> Result<()> {
        let mut symbols = Vec::with_capacity(data.len() / 2 + 1);

        // 2er Schritte durch das Array:
        for chunk in data.chunks(2) {
            // Mark setzen:
            let mark_val = chunk[0];
            // Letzten Chunk mit nur einem Wert abfangen:
            let space_val = if chunk.len() > 1 {chunk[1]} else {0};

            // Pulse je nach Wert setzen:
            let mark = Pulse::new(PinState::High, PulseTicks::new(mark_val)?);
            let space = Pulse::new(PinState::Low, PulseTicks::new(space_val)?);

            symbols.push(Symbol::new(mark, space));
        }
        let mut encoder = CopyEncoder::new()?;
        let tx_config = TransmitConfig::default();

        self.tx_driver.send_and_wait(&mut encoder, &symbols, &tx_config)?;

        Ok(())
    }
}