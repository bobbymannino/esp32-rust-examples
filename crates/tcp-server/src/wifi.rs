use anyhow::{Context as _, Result, anyhow};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::peripherals::Peripherals,
    nvs::EspDefaultNvsPartition,
    wifi::{AccessPointInfo, AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi, PmfConfiguration},
};

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

    wifi.set_configuration(&Configuration::Client(ClientConfiguration::default()))?;
    wifi.start()?;
    log::info!("WiFi started, scanning for {ssid}");

    let access_point = find_access_point(&mut wifi, ssid)?;
    let auth_method = negotiate_auth_method(access_point.as_ref(), ssid);

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ssid
            .try_into()
            .map_err(|_| anyhow!("SSID must be at most 32 bytes, {ssid} is {}", ssid.len()))?,
        password: password
            .try_into()
            .map_err(|_| anyhow!("password must be at most 64 bytes, this one is {}", password.len()))?,
        auth_method,
        pmf_cfg: pmf_for(auth_method),
        // Knowing the channel up front lets the driver skip straight to it instead of sweeping all of them.
        channel: access_point.as_ref().map(|ap| ap.channel),
        ..Default::default()
    }))?;

    log::info!("Connecting to {ssid} using {auth_method:?}");
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

/// Scans the air for [`SSID`] so the security the access point actually advertises can be used.
///
/// Returns `None` when the network is not on the air; the connection is still attempted in that case, since the
/// access point may simply be hiding its SSID in its beacons.
fn find_access_point(wifi: &mut BlockingWifi<EspWifi<'_>>, ssid: &str) -> Result<Option<AccessPointInfo>> {
    let access_point = wifi
        .scan()
        .context("scan failed")?
        .into_iter()
        .filter(|ap| ap.ssid.as_str() == ssid)
        .max_by_key(|ap| ap.signal_strength);

    match &access_point {
        Some(ap) => log::info!(
            "Found {ssid} on channel {} at {} dBm advertising {:?}",
            ap.channel,
            ap.signal_strength,
            ap.auth_method
        ),
        None => log::warn!("{ssid} was not seen in the scan, it may be a hidden network"),
    }

    Ok(access_point)
}

/// Picks the authentication method to connect with.
///
/// The access point decides which of WPA2 and WPA3 is on offer, so prefer whatever it advertises. Falling back to
/// [`AuthMethod::WPA2WPA3Personal`] keeps a hidden network working, because that setting is a
/// *minimum*: the driver accepts WPA2 and WPA3 access points, and refuses anything weaker.
fn negotiate_auth_method(access_point: Option<&AccessPointInfo>, ssid: &str) -> AuthMethod {
    match access_point.and_then(|ap| ap.auth_method) {
        Some(AuthMethod::None) => {
            log::warn!("{ssid} is an open network, but this example is built for WPA2/WPA3");
            AuthMethod::WPA2WPA3Personal
        }
        Some(auth_method) => auth_method,
        None => AuthMethod::WPA2WPA3Personal,
    }
}

/// Decides how to advertise Protected Management Frames, which WPA3 is built on top of.
///
/// PMF stops an attacker forging the unencrypted management frames that WPA2 leaves in the clear, which is what makes
/// deauthentication attacks possible. WPA3 makes it mandatory, so require it there. Anywhere else only advertise it:
/// requiring PMF against a WPA2-only access point that does not support it means no connection at all.
fn pmf_for(auth_method: AuthMethod) -> PmfConfiguration {
    PmfConfiguration::Capable {
        required: auth_method == AuthMethod::WPA3Personal,
    }
}
