use esp_idf_svc::hal::{
    delay::FreeRtos,
    i2c::{I2cConfig, I2cDriver},
    peripherals::Peripherals,
    units::Hertz,
};

use crate::adxl345::ADXL345;

mod adxl345;

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

    // SDA = Serial Data
    let sda = peripherals.pins.gpio21;
    // SCL = Serial Clock
    let scl = peripherals.pins.gpio22;

    let config = I2cConfig::new()
        .baudrate(Hertz(400_000))
        .sda_enable_pullup(true)
        .scl_enable_pullup(true);
    let i2c = I2cDriver::new(peripherals.i2c0, sda, scl, &config)?;

    let mut adxl345 = ADXL345::new(i2c)?;
    log::info!("ADXL345 ready");

    loop {
        let (x, y, z) = adxl345.read_raw()?;
        log::info!("x: {}, y: {}, z: {}", x, y, z);
        FreeRtos::delay_ms(100);
    }
}
