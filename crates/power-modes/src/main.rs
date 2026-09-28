use std::{convert::Infallible, time::Duration};

use esp_idf_svc::{
    hal::{
        gpio::{Input, Level, PinDriver, RtcInput},
        sleep::{DeepSleep, LightSleep, RtcWakeLevel},
    },
    sys::{self, EspError, esp},
};

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
    todo!();
}

/// Keep everything powered by turning off Wi-Fi power saving, so the radio
/// never sleeps between beacons.
///
/// Wi-Fi must already be started.
#[allow(dead_code)]
fn active() -> Result<(), EspError> {
    esp!(unsafe { sys::esp_wifi_set_ps(sys::wifi_ps_type_t_WIFI_PS_NONE) })
}

/// Let the radio sleep between DTIM beacons while the CPU keeps running. The
/// station stays connected to the access point.
///
/// Wi-Fi must already be started.
#[allow(dead_code)]
fn modem_sleep() -> Result<(), EspError> {
    esp!(unsafe { sys::esp_wifi_set_ps(sys::wifi_ps_type_t_WIFI_PS_MIN_MODEM) })
}

/// Pause the CPU for `duration`. RAM is retained, so execution carries on from
/// here once the timer wakes the chip.
#[allow(dead_code)]
fn light_sleep(duration: Duration) -> Result<(), EspError> {
    LightSleep::new()?.wakeup_on_timer(duration)?.enter()
}

/// Power off everything except the RTC domain for `duration`. Waking up is a
/// full reboot, so this only returns if setting up the wake up timer fails.
#[allow(dead_code)]
fn deep_sleep(duration: Duration) -> Result<Infallible, EspError> {
    DeepSleep::new()?.wakeup_on_timer(duration)?.enter()
}

/// Pause the CPU until `pin` changes level, e.g. the INT1 pin of an ADXL345
/// firing on activity. RAM is retained, so execution carries on from here once
/// the chip wakes.
///
/// The ESP32 can only wake on a level, not an edge, so this wakes on whichever
/// level the pin is not currently at. The sensor has to release its interrupt
/// (for an ADXL345, read `INT_SOURCE`) before sleeping again, otherwise the
/// chip wakes straight back up.
#[allow(dead_code)]
fn light_sleep_until_pin_change(pin: &PinDriver<Input>) -> Result<(), EspError> {
    LightSleep::new()?.wakeup_on_gpio(pin, !pin.get_level())?.enter()
}

/// Power off everything except the RTC domain until `pin` changes level, e.g.
/// the INT1 pin of an ADXL345 firing on activity. Waking up is a full reboot,
/// so this only returns if setting up the wake up pin fails.
///
/// Only RTC GPIOs can wake the chip from deep sleep (on the ESP32: 0, 2, 4,
/// 12-15, 25-27 and 32-39), which is why the pin must be an `RtcInput`. Like
/// light sleep this wakes on a level, so it wakes on whichever level the pin is
/// not currently at.
#[allow(dead_code)]
fn deep_sleep_until_pin_change(pin: &PinDriver<RtcInput>) -> Result<Infallible, EspError> {
    let level = match pin.get_level() {
        Level::Low => RtcWakeLevel::AnyHigh,
        Level::High => RtcWakeLevel::AllLow,
    };

    DeepSleep::new()?.wakeup_on_rtc(pin, level)?.enter()
}

/// Deep sleep with the RTC peripherals and RTC memory also powered off, leaving
/// only the RTC timer running. Waking up is a full reboot, so this only returns
/// if powering down a domain or setting up the wake up timer fails.
///
/// There is no safe wrapper for `esp_sleep_pd_config`, so that part is still
/// `unsafe`.
#[allow(dead_code)]
fn hibernate(duration: Duration) -> Result<Infallible, EspError> {
    let domains = [
        sys::esp_sleep_pd_domain_t_ESP_PD_DOMAIN_RTC_PERIPH,
        sys::esp_sleep_pd_domain_t_ESP_PD_DOMAIN_RTC_SLOW_MEM,
        sys::esp_sleep_pd_domain_t_ESP_PD_DOMAIN_RTC_FAST_MEM,
        sys::esp_sleep_pd_domain_t_ESP_PD_DOMAIN_XTAL,
    ];

    for domain in domains {
        esp!(unsafe { sys::esp_sleep_pd_config(domain, sys::esp_sleep_pd_option_t_ESP_PD_OPTION_OFF) })?;
    }

    deep_sleep(duration)
}
