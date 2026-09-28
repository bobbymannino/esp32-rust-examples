use anyhow::{Context, anyhow};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::modem::WifiModemPeripheral,
    nvs::EspDefaultNvsPartition,
    wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi, PmfConfiguration},
};

/// Joins a WPA2 or WPA3 network as a station and blocks until DHCP has handed out an IP address.
///
/// See the `wifi-wpa` project for a fuller version that scans for the network and reconnects when it drops.
pub fn connect<'d, M: WifiModemPeripheral + 'd>(
    modem: M,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
    ssid: &str,
    password: &str,
) -> anyhow::Result<BlockingWifi<EspWifi<'d>>> {
    let mut wifi = BlockingWifi::wrap(EspWifi::new(modem, sysloop.clone(), Some(nvs))?, sysloop)?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ssid
            .try_into()
            .map_err(|_| anyhow!("SSID must be at most 32 bytes, {ssid:?} is {}", ssid.len()))?,
        password: password
            .try_into()
            .map_err(|_| anyhow!("password must be at most 64 bytes, this one is {}", password.len()))?,
        // This is a minimum, so the driver accepts both WPA2 and WPA3 access points and refuses anything weaker.
        auth_method: AuthMethod::WPA2WPA3Personal,
        // WPA3 needs Protected Management Frames, but requiring them would lock out WPA2 access points without it.
        pmf_cfg: PmfConfiguration::Capable { required: false },
        ..Default::default()
    }))?;

    wifi.start()?;
    log::info!("Connecting to {ssid}");
    wifi.connect().context("failed to associate with the access point")?;
    // Association only gets us onto the link. DHCP runs afterwards, so wait for the interface to actually be up.
    wifi.wait_netif_up().context("associated but never got an IP address")?;

    Ok(wifi)
}
