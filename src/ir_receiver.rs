use esp_idf_svc::hal::gpio::{AnyIOPin, Input, PinDriver};
use infrared::protocol::Nec;
use infrared::{PeriodicPoll, Receiver};
use infrared::cmd::AddressCommand;

// IrReceiver als Struct bauen
pub struct IrReceiver<'a> {
    // Pin und Decoder in einem definieren:
    decoder: PeriodicPoll<Nec, PinDriver<'a, Input>>
}

impl<'a> IrReceiver<'a> {
    pub fn new(pin: PinDriver<'a, Input>) -> Self {
        Self {
            // Pin direkt an die IR Bibliothek:
            decoder: PeriodicPoll::with_pin(20_000, pin)
        }
    }

    pub fn listen(&mut self) -> Option<u32> {
        // Signal auslesen und decodieren:
        if let Ok(Some(cmd)) = self.decoder.poll() {
            return Some(cmd.command());
        }
        // Noch kein Signal oder Fehler:
        None
    }
}