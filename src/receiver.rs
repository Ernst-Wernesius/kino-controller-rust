use std::io::Write;
use std::net::TcpStream;
use esp_idf_svc::hal::delay::FreeRtos;
use anyhow::Result;

pub struct Receiver {
    address: String
}

impl Receiver {
    // Konstruktor:
    pub fn new(ip: &str, port: u16) -> Self {
        Self {
            address: format!("{}:{}", ip, port)
        }
    }

    pub fn turn_on(&self) -> Result<()> {
        self.send_command("PO")?;
        FreeRtos::delay_ms(1000);
        self.send_command("49FN")?;
        Ok(())
    }

    pub fn turn_off(&self) -> Result<()> {
        self.send_command("PF")?;
        Ok(())
    }

    fn send_command(&self, command: &str) -> Result <()> {
        // TCP Verbindung aufbauen:
        let mut stream = TcpStream::connect(&self.address)?;

        // String bauen:
        let payload = format!("{}\r\n", command);

        // Als Byte senden:
        stream.write_all(payload.as_bytes())?;

        Ok(())
    }
}