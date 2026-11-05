use std::time::{Duration, Instant};

use anyhow::Result;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::rmt::config::TransmitConfig;
use esp_idf_svc::hal::rmt::encoder::CopyEncoder;
use esp_idf_svc::hal::rmt::{PinState, Pulse, PulseTicks, Symbol, TxChannelDriver};

// Infrarot Daten:
const IR_DATA_SCREEN_DOWN: [u16; 50] = [
    460, 1330, 1400, 410, 480, 1300, 1330, 460, 440, 1350, 1360, 430, 440, 1350, 1330, 460, 440,
    1350, 1360, 430, 440, 1350, 1380, 410, 440, 1350, 1360, 430, 440, 1350, 1380, 410, 440, 1350,
    480, 1300, 480, 1300, 430, 1350, 440, 1350, 1380, 410, 480, 1300, 480, 1300, 430, 13370,
];
const IR_DATA_SCREEN_UP: [u16; 50] = [
    488, 1299, 1385, 402, 492, 1296, 1388, 400, 490, 1297, 1382, 405, 490, 1297, 1386, 401, 490,
    1297, 1382, 405, 490, 1297, 1382, 405, 489, 1298, 1382, 403, 491, 1297, 1383, 403, 491, 1297,
    488, 1297, 492, 1295, 490, 1297, 492, 1295, 489, 1297, 1387, 400, 490, 1293, 492, 13338,
];

pub struct Screen<'a> {
    tx_driver: TxChannelDriver<'a>,
    blocked_until: Option<Instant>,
}

impl<'a> Screen<'a> {
    // Konstruktor:
    pub fn new(tx_driver: TxChannelDriver<'a>) -> Self {
        Self {
            tx_driver,
            blocked_until: None,
        }
    }

    // Runterfahren:
    pub fn move_down(&mut self) -> Result<()> {
        self.check_blocked()?;
        for _ in 0..3 {
            self.send_raw(&IR_DATA_SCREEN_DOWN)?;
            FreeRtos::delay_ms(14);
        }
        self.blocked_until = Some(Instant::now() + Duration::from_secs(15));
        Ok(())
    }

    // Hochfahren:
    pub fn move_up(&mut self) -> Result<()> {
        self.check_blocked()?;
        for _ in 0..3 {
            self.send_raw(&IR_DATA_SCREEN_UP)?;
            FreeRtos::delay_ms(14);
        }
        self.blocked_until = Some(Instant::now() + Duration::from_secs(15));
        Ok(())
    }

    fn send_raw(&mut self, data: &[u16]) -> Result<()> {
        let mut symbols = Vec::with_capacity(data.len() / 2 + 1);

        // 2er Schritte durch das Array:
        for chunk in data.chunks(2) {
            // Mark setzen:
            let mark_val = chunk[0];
            // Letzten Chunk mit nur einem Wert abfangen:
            let space_val = if chunk.len() > 1 { chunk[1] } else { 0 };

            // Pulse je nach Wert setzen:
            let mark = Pulse::new(PinState::High, PulseTicks::new(mark_val)?);
            let space = Pulse::new(PinState::Low, PulseTicks::new(space_val)?);

            symbols.push(Symbol::new(mark, space));
        }
        let mut encoder = CopyEncoder::new()?;
        let tx_config = TransmitConfig::default();

        self.tx_driver
            .send_and_wait(&mut encoder, &symbols, &tx_config)?;

        Ok(())
    }

    fn check_blocked(&self) -> Result<()> {
        if let Some(until) = self.blocked_until {
            let now = Instant::now();
            if now < until {
                let rest = until.duration_since(now).as_secs_f32();
                anyhow::bail!("Screen blocked for another {:.1} s", rest);
            }
        }
        Ok(())
    }
}
