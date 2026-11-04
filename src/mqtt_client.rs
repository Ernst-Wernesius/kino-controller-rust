use std::sync::mpsc::Sender;

use esp_idf_svc::mqtt::client::{EspMqttConnection, EventPayload};

use crate::event::{BeamerCommand, Event, ScreenDirection};

pub const TOPIC_STATUS: &str = "cinema/esp/status";
pub const TOPIC_RII: &str = "cinema/rii/button";
pub const TOPIC_BEAMER_SET: &str = "cinema/beamer/set";
pub const TOPIC_SCREEN_SET: &str = "cinema/screen/set";

pub fn spawn_mqtt_listener(
    mut connection: EspMqttConnection,
    tx: Sender<Event>,
) -> anyhow::Result<()> {
    // Thread für den MQTT Listener:
    std::thread::Builder::new()
        .stack_size(6 * 1024)
        .spawn(move || {
            while let Ok(event) = connection.next() {
                match event.payload() {
                    EventPayload::Connected(_) => {
                        let _ = tx.send(Event::Connected);
                    }
                    EventPayload::Received { topic, data, .. } => match (topic, data) {
                        (Some(TOPIC_BEAMER_SET), b"ON") => {
                            let _ = tx.send(Event::Beamer(BeamerCommand::On));
                        }
                        (Some(TOPIC_BEAMER_SET), b"OFF") => {
                            let _ = tx.send(Event::Beamer(BeamerCommand::Off));
                        }
                        (Some(TOPIC_SCREEN_SET), b"UP") => {
                            let _ = tx.send(Event::Screen(ScreenDirection::Up));
                        }
                        (Some(TOPIC_SCREEN_SET), b"DOWN") => {
                            let _ = tx.send(Event::Screen(ScreenDirection::Down));
                        }
                        _ => {
                            let _ = tx.send(Event::Log(format!(
                                "{:?}, {:?}",
                                topic,
                                String::from_utf8_lossy(data)
                            )));
                        }
                    },
                    other => {
                        let _ = tx.send(Event::Log(format!("{:?}", other)));
                    }
                }
            }
            // Hier wenn die Verbindung beendet wurde
        })?;
    Ok(())
}
