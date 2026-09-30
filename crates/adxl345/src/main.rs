mod adxl345;

use esp_idf_svc::hal::{
    delay::FreeRtos,
    gpio::{Level, PinDriver, Pull},
    i2c::{I2cConfig, I2cDriver},
    peripherals::Peripherals,
    sleep::LightSleep,
    units::Hertz,
};

use crate::adxl345::ADXL345;

/// The threshold to trigger an interrupt, in mg
const MG_THRESHOLD: i16 = 1_000;

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
    let int_pin = PinDriver::input(peripherals.pins.gpio23, Pull::Down)?;

    loop {
        let Ok((x, y, z)) = adxl345.read_raw() else {
            log::error!("Failed to read measurements");
            FreeRtos::delay_ms(100);
            continue;
        };
        log::info!("x: {}, y: {}, z: {}", x, y, z);
        if x.abs() > MG_THRESHOLD || y.abs() > MG_THRESHOLD || z.abs() > MG_THRESHOLD {
            FreeRtos::delay_ms(100);
        } else {
            log::info!("Enabling interupt");
            adxl345.enable_interupt_activity(f32::from(MG_THRESHOLD))?;
            match int_pin.get_level() {
                Level::High => log::info!("Interrupt pin set to high"),
                Level::Low => log::info!("Interrupt pin set to low"),
            }
            log::info!("Putting into light sleep");
            FreeRtos::delay_ms(100);
            match LightSleep::new()?.wakeup_on_gpio(&int_pin, Level::High)?.enter() {
                Ok(()) => log::info!("Woke up from light sleep"),
                Err(error) => log::error!("Failed to light sleep: {error}"),
            }
            match adxl345.get_interrupt_source() {
                Ok(source) => log::info!("Interrupt source: {:?}", source),
                Err(error) => log::error!("{error}"),
            }
            FreeRtos::delay_ms(100);
        }
    }
}
