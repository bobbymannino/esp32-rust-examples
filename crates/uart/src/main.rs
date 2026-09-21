use std::time::Duration;

use esp_idf_svc::{
    hal::{
        delay::{FreeRtos, TickType},
        gpio,
        peripherals::Peripherals,
        uart::{UartDriver, config::Config},
        units::Hertz,
    },
    sys::EspError,
};

fn main() -> Result<(), EspError> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;

    let tx = peripherals.pins.gpio1;
    let rx = peripherals.pins.gpio3;

    let config = Config::new().baudrate(Hertz(115_200));

    let uart = UartDriver::new(
        peripherals.uart0,
        tx,
        rx,
        Option::<gpio::AnyIOPin>::None,
        Option::<gpio::AnyIOPin>::None,
        &config,
    )?;

    log::info!("UART ready");
    loop {
        let mut buf = [0_u8; 64];
        let timeout = TickType::from(Duration::from_secs(1)).ticks();
        match uart.read(&mut buf, timeout) {
            Ok(0) => log::info!("No bytes read"),
            Err(e) => log::error!("Read error: {e:?}"),
            Ok(len) => {
                log::info!("Read {len} bytes:");
                log::info!("  {}", String::from_utf8_lossy(&buf));
            }
        }

        // TODO somehow read back commands from device.
    }
}
