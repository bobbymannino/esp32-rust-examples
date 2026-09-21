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
        match uart.write(&[0x01]) {
            Ok(len) => log::info!("{len} bytes written successfully"),
            Err(e) => log::error!("write error: {e:?}"),
        }

        FreeRtos::delay_ms(500);

        let timeout = TickType::from(Duration::from_secs(1)).ticks();
        let mut buf = [0_u8; 64];
        match uart.read(&mut buf, timeout) {
            Ok(0) => log::error!("no bytes read"),
            Ok(len) => log::info!("{len} bytes read successfully"),
            Err(e) => log::error!("read error: {e:?}"),
        }

        FreeRtos::delay_ms(500);
    }
}
