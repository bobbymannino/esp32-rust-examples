use anyhow::{Context as _, Result, anyhow};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::peripherals::Peripherals,
    nvs::EspDefaultNvsPartition,
    sys::{esp, esp_wifi_set_ps, wifi_ps_type_t_WIFI_PS_NONE},
    wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi, PmfConfiguration},
};

/// Joins a WPA2 or WPA3 network as a station and blocks until DHCP has handed out an IP address.
pub fn connect<'a>(
    peripherals: Peripherals,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
    ssid: &str,
    password: &str,
) -> Result<BlockingWifi<EspWifi<'a>>> {
    // Without the `BlockingWifi` wrapper the function calls will return immediately (before completing).
    // Normally this would be on a worker thread so it would not be blocking anything else.
    let mut wifi = BlockingWifi::wrap(EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs))?, sysloop)?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ssid
            .try_into()
            .map_err(|_| anyhow!("SSID must be at most 32 bytes, {ssid} is {}", ssid.len()))?,
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

    // By default the radio sleeps between beacons to save power, and can miss the broadcast ARP requests other
    // devices send to find it. When that happens it can still reach out, but nothing can connect in.
    esp!(unsafe { esp_wifi_set_ps(wifi_ps_type_t_WIFI_PS_NONE) }).context("failed to disable WiFi power saving")?;

    log::info!("Connecting to {ssid}");
    wifi.connect().context("failed to associate with the access point")?;
    // Association only gets us onto the link. DHCP runs afterwards, so wait for the interface to actually be up.
    wifi.wait_netif_up().context("associated but never got an IP address")?;
    log_connection(&wifi, ssid)?;
    Ok(wifi)
}

/// Logs the addressing the network handed out over DHCP.
fn log_connection(wifi: &BlockingWifi<EspWifi<'_>>, ssid: &str) -> Result<()> {
    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;

    log::info!("Connected to {ssid}");
    log::info!("  IP address: {}", ip_info.ip);
    log::info!("  Gateway:    {}", ip_info.subnet.gateway);
    log::info!("  DNS:        {:?}", ip_info.dns);

    Ok(())
}
