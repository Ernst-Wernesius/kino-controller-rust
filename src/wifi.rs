use esp_idf_svc::wifi::{BlockingWifi, EspWifi, Configuration, ClientConfiguration};
use anyhow::Result;

pub fn connect_wifi(
    wifi: &mut BlockingWifi<EspWifi<'static>>,
    ssid: &str,
    password: &str,
) -> Result<()> {
    let wifi_config = Configuration::Client(ClientConfiguration {
        ssid: ssid.try_into().unwrap(),
        password: password.try_into().unwrap(),
        ..Default::default()
    });

    wifi.set_configuration(&wifi_config)?;
    wifi.start()?;
    wifi.connect()?;
    wifi.wait_netif_up()?;

    Ok(())
}