mod wifi;

use std::time::{SystemTime, UNIX_EPOCH};

use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{delay::FreeRtos, peripherals::Peripherals},
    http::{
        Method,
        server::{Configuration as HttpConfiguration, EspHttpServer},
    },
    io::Write,
    nvs::EspDefaultNvsPartition,
    sys::{settimeofday, time_t, timeval},
};

/// Credentials are baked in at compile time so they never have to live in the repository.
///
/// ```sh
/// WIFI_SSID="My Network" WIFI_PASSWORD="hunter2" cargo run
/// ```
const SSID: &str = env!("WIFI_SSID", "set WIFI_SSID to the network to join");
const PASSWORD: &str = env!("WIFI_PASSWORD", "set WIFI_PASSWORD to the network's password");

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    match run() {
        Ok(()) => {}
        Err(e) => {
            eprint!("{}", e);
            std::process::exit(1);
        }
    }
}

fn run() -> anyhow::Result<()> {
    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // Both of these stop working when dropped, so they have to stay alive for as long as the server should run.
    let wifi = wifi::connect(peripherals.modem, sysloop, nvs, SSID, PASSWORD)?;
    let mut server = EspHttpServer::new(&HttpConfiguration::default())?;

    server.fn_handler("/api/rtc", Method::Get, |req| {
        let body = format!(r#"{{"rtc":{}}}"#, rtc_seconds());
        req.into_response(200, None, &[("Content-Type", "application/json")])?
            .write_all(body.as_bytes())
    })?;

    let ip = wifi.wifi().sta_netif().get_ip_info()?.ip;
    log::info!("Listening on http://{ip}/api/rtc");

    // The server runs on its own task, so this one only has to keep `wifi` and `server` from being dropped.
    loop {
        FreeRtos::delay_ms(u32::MAX);
    }
}

/// Reads the RTC as whole seconds since the Unix epoch (UTC).
///
/// Nothing sets the clock, so it starts from 0 at boot. It only holds the real time once something has set it
fn rtc_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Sets the RTC to `seconds` since the Unix epoch (UTC).
fn set_rtc_seconds(seconds: u64) -> anyhow::Result<()> {
    let time = timeval {
        tv_sec: time_t::try_from(seconds)?,
        tv_usec: 0,
    };
    esp_idf_svc::sys::esp!(unsafe { settimeofday(&time, std::ptr::null()) })?;
    Ok(())
}
