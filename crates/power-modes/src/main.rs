use esp_idf_svc::{
    hal::sleep::{DeepSleep, LightSleep},
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

