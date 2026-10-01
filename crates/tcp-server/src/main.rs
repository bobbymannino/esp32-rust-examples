use esp_idf_svc::{eventloop::EspSystemEventLoop, hal::peripherals::Peripherals, nvs::EspDefaultNvsPartition};

mod tcp;
mod wifi;

/// Credentials are baked in at compile time so they never have to live in the repository.
///
/// ```sh
/// WIFI_SSID="My Network" WIFI_PASSWORD="hunter2" PORT=8080 cargo run
/// ```
const SSID: &str = env!("WIFI_SSID", "set WIFI_SSID to the network to join");
const PASSWORD: &str = env!("WIFI_PASSWORD", "set WIFI_PASSWORD to the network's password");
const PORT: u16 = env!("PORT", "Set PORT to the desired port number")
    .parse()
    .expect("PORT should be a valid u16");

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    if let Err(e) = run() {
        eprint!("{}", e);
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // WiFi disconnects when this is dropped, so it has to stay alive for as long as the server runs.
    let wifi = wifi::connect(peripherals, sysloop, nvs, SSID, PASSWORD)?;

    let ip = wifi.wifi().sta_netif().get_ip_info()?.ip;
    log::info!("Listening on {ip}:{PORT}, try `nc {ip} {PORT}`");

    // Only returns if the listener itself fails, so `wifi` stays alive for the life of the server.
    tcp::serve(PORT)
}
