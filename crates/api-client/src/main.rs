use anyhow::{Context, anyhow};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{delay::FreeRtos, peripherals::Peripherals},
    http::{
        Method,
        client::{Configuration as HttpConfiguration, EspHttpConnection},
    },
    nvs::EspDefaultNvsPartition,
    wifi::{AccessPointInfo, AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi, PmfConfiguration},
};

/// Credentials are baked in at compile time so they never have to live in the repository.
///
/// ```sh
/// WIFI_SSID="My Network" WIFI_PASSWORD="hunter2" IP="192.168.1.1" PORT=80 cargo run
/// ```
const SSID: &str = env!("WIFI_SSID", "set WIFI_SSID to the network to join");
const PASSWORD: &str = env!("WIFI_PASSWORD", "set WIFI_PASSWORD to the network's password");
const IP: &str = env!("IP", "set IP to the IP address of the API server");
const PORT: u16 = match u16::from_str_radix(env!("PORT", "set PORT to the port of the API server"), 10) {
    Ok(port) => port,
    Err(_) => panic!("PORT must be a number between 0 and 65535"),
};

/// Time between HTTP requests
const POLL_GAP_MS: u32 = 5_000;

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    match run() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

fn run() -> anyhow::Result<()> {
    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = BlockingWifi::wrap(EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs))?, sysloop)?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration::default()))?;
    wifi.start()?;
    log::info!("WiFi started, scanning for {SSID}");

    let access_point = find_access_point(&mut wifi)?;
    let auth_method = negotiate_auth_method(access_point.as_ref());

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: SSID
            .try_into()
            .map_err(|_| anyhow!("SSID must be at most 32 bytes, {SSID:?} is {}", SSID.len()))?,
        password: PASSWORD
            .try_into()
            .map_err(|_| anyhow!("password must be at most 64 bytes, this one is {}", PASSWORD.len()))?,
        auth_method,
        pmf_cfg: pmf_for(auth_method),
        channel: access_point.as_ref().map(|ap| ap.channel),
        ..Default::default()
    }))?;

    log::info!("Connecting to {SSID} using {auth_method:?}");
    wifi.connect().context("failed to associate with the access point")?;
    // Association only gets us onto the link. DHCP runs afterwards, so wait for the interface to actually be up.
    wifi.wait_netif_up().context("associated but never got an IP address")?;
    log_connection(&wifi)?;

    let mut client = ApiClient::new(IP, PORT)?;

    for count in 0u32.. {
        let body = format!("hello from the ESP32, message #{count}");
        // A single failed request (server restarting, packet loss) should not bring the whole device down.
        match client.post("/api/post", &body) {
            Ok(status) => log::info!("POST #{count} returned {status}"),
            Err(e) => log::error!("POST #{count} failed: {e:?}"),
        }
        FreeRtos::delay_ms(POLL_GAP_MS);
    }

    Ok(())
}

/// A tiny HTTP client for the API server.
///
/// The underlying connection is reused between requests, so the TCP socket is kept alive rather than reopened for
/// every call.
struct ApiClient {
    connection: EspHttpConnection,
    base_url: String,
}

impl ApiClient {
    fn new(ip: &str, port: u16) -> anyhow::Result<Self> {
        let connection = EspHttpConnection::new(&HttpConfiguration {
            timeout: Some(core::time::Duration::from_secs(10)),
            ..Default::default()
        })
        .context("failed to create the HTTP connection")?;

        Ok(Self {
            connection,
            base_url: format!("http://{ip}:{port}"),
        })
    }

    /// Sends `body` as plain text to `path` and returns the response's status code.
    fn post(&mut self, path: &str, body: &str) -> anyhow::Result<u16> {
        let url = format!("{}{path}", self.base_url);
        // Giving the length up front stops the client falling back to chunked transfer encoding.
        let content_length = body.len().to_string();
        let headers = [
            ("Content-Type", "text/plain"),
            ("Content-Length", content_length.as_str()),
        ];

        self.connection
            .initiate_request(Method::Post, &url, &headers)
            .with_context(|| format!("failed to open a request to {url}"))?;
        self.connection
            .write_all(body.as_bytes())
            .context("failed to send the request body")?;
        self.connection
            .initiate_response()
            .context("failed to read the response")?;

        Ok(self.connection.status())
    }
}

/// Scans the air for [`SSID`] so the security the access point actually advertises can be used.
///
/// Returns `None` when the network is not on the air; the connection is still attempted in that case, since the
/// access point may simply be hiding its SSID in its beacons.
fn find_access_point(wifi: &mut BlockingWifi<EspWifi<'_>>) -> anyhow::Result<Option<AccessPointInfo>> {
    let access_point = wifi
        .scan()
        .context("scan failed")?
        .into_iter()
        .filter(|ap| ap.ssid.as_str() == SSID)
        .max_by_key(|ap| ap.signal_strength);

    match &access_point {
        Some(ap) => log::info!(
            "Found {SSID} on channel {} at {} dBm advertising {:?}",
            ap.channel,
            ap.signal_strength,
            ap.auth_method
        ),
        None => log::warn!("{SSID} was not seen in the scan, it may be a hidden network"),
    }

    Ok(access_point)
}

/// Picks the authentication method to connect with.
///
/// The access point decides which of WPA2 and WPA3 is on offer, so prefer whatever it advertises. Falling back to
/// [`AuthMethod::WPA2WPA3Personal`] keeps a hidden network working, because that setting is a
/// *minimum*: the driver accepts WPA2 and WPA3 access points, and refuses anything weaker.
fn negotiate_auth_method(access_point: Option<&AccessPointInfo>) -> AuthMethod {
    match access_point.and_then(|ap| ap.auth_method) {
        Some(AuthMethod::None) => {
            log::warn!("{SSID} is an open network, but this example is built for WPA2/WPA3");
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

/// Logs the addressing the network handed out over DHCP.
fn log_connection(wifi: &BlockingWifi<EspWifi<'_>>) -> anyhow::Result<()> {
    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;

    log::info!("Connected to {SSID}");
    log::info!("  IP address: {}", ip_info.ip);
    log::info!("  Gateway:    {}", ip_info.subnet.gateway);
    log::info!("  DNS:        {:?}", ip_info.dns);

    Ok(())
}
