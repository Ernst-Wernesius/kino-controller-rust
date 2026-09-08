use esp_idf_svc::hal::rmt::{PinState, Pulse, PulseTicks, Symbol, TxChannelDriver};
use esp_idf_svc::hal::rmt::config::TransmitConfig;
use esp_idf_svc::hal::rmt::encoder::CopyEncoder;
use anyhow::Result;

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
        self.send_raw(&IR_DATA_SCREEN_DOWN)
    }

    // Hochfahren:
    pub fn move_up(&mut self) -> Result<()> {
        self.send_raw(&IR_DATA_SCREEN_UP)
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