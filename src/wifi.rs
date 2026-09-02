use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi, Configuration, ClientConfiguration};
use anyhow::Result;

pub fn connect_wifi(
    ssid: &str,
    password: &str,
    modem: Modem,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition
) -> Result<BlockingWifi<EspWifi<'static>>> {
    // WLAN-Treiber initialisieren:
    let esp_wifi = EspWifi::new(modem, sysloop.clone(), Some(nvs))?;

    // Wrapper bauen:
    let mut wifi = BlockingWifi::wrap(esp_wifi, sysloop)?;

    // WLAN Konfiguration schreiben:
    let wifi_config = Configuration::Client(ClientConfiguration {
        ssid: ssid.try_into().unwrap(),
        password: password.try_into().unwrap(),
        ..Default::default()
    });

    // Einstellungen anwenden und Modul starten
    wifi.set_configuration(&wifi_config)?;
    wifi.start()?;

    // WLAN connection herstellen:
    wifi.connect()?;
    // Wartem auf IP Zuweisung
    wifi.wait_netif_up()?;

    // Wlan Objekt zurückgeben:
    Ok(wifi)
}