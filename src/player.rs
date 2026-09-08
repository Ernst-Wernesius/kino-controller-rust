use std::net::UdpSocket;
use anyhow::Result;

pub struct Player {
    // Mac-Adresse hier:
    mac: [u8; 6]
}

impl Player {
    pub fn new(mac: [u8; 6]) -> Self {
        Self {mac}
    }

    pub fn wake_up(&self) -> Result<()> {
        // Paket zusammenbauen:
        let mut packet = vec![0xFF; 6];
        // Mac Adresse anhängen:
        for _ in 0..16 {
            packet.extend_from_slice(&self.mac);
        }

        // UDP-Socket:
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_broadcast(true)?;

        // Paket senden:
        socket.send_to(&packet, "255.255.255.255:9")?;

        Ok(())
    }
}