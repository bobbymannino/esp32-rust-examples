mod wifi;

use std::time::{SystemTime, UNIX_EPOCH};

use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{delay::FreeRtos, peripherals::Peripherals},
    http::{
        Method,
        server::{Configuration as HttpConfiguration, EspHttpConnection, EspHttpServer, Request},
    },
    io::Write,
    nvs::EspDefaultNvsPartition,
    sys::{settimeofday, time_t, timeval},
};

/// Longest body accepted when setting the RTC. A `u64` is at most 20 digits, which leaves room for whitespace.
const MAX_RTC_BODY_BYTES: usize = 32;

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

    server.fn_handler("/api/rtc", Method::Post, |mut req| -> anyhow::Result<()> {
        // Ignore parameters such as `; charset=utf-8`, only the media type matters.
        let is_text = req
            .header("Content-Type")
            .and_then(|ct| ct.split(';').next())
            .is_some_and(|ct| ct.trim().eq_ignore_ascii_case("text/plain"));
        if !is_text {
            req.into_status_response(415)?
                .write_all(b"Content-Type must be text/plain")?;
            return Ok(());
        }

        let mut buf = [0u8; MAX_RTC_BODY_BYTES];
        let Some(len) = read_body(&mut req, &mut buf)? else {
            req.into_status_response(413)?.write_all(b"body is too long")?;
            return Ok(());
        };

        let Some(seconds) = str::from_utf8(&buf[..len]).ok().and_then(|s| s.trim().parse().ok()) else {
            req.into_status_response(400)?
                .write_all(b"body must be whole seconds since the Unix epoch")?;
            return Ok(());
        };

        set_rtc_seconds(seconds)?;
        log::info!("RTC set to {seconds}");

        let body = format!(r#"{{"rtc":{}}}"#, rtc_seconds());
        req.into_response(200, None, &[("Content-Type", "application/json")])?
            .write_all(body.as_bytes())?;
        Ok(())
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

/// Reads the whole request body into `buf`, returning how many bytes were read, or `None` if it does not fit.
fn read_body(req: &mut Request<&mut EspHttpConnection<'_>>, buf: &mut [u8]) -> anyhow::Result<Option<usize>> {
    let mut len = 0;
    while len < buf.len() {
        match req.read(&mut buf[len..])? {
            0 => return Ok(Some(len)),
            n => len += n,
        }
    }

    // The buffer is full, so the body only fits if there is nothing left to read.
    Ok((req.read(&mut [0u8; 1])? == 0).then_some(len))
}
