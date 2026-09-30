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

    let mut count = 0;
    loop {
        let (x, y, z) = adxl345.read_raw()?;
        log::info!("x: {}, y: {}, z: {}", x, y, z);
        FreeRtos::delay_ms(100);
        count += 1;

        if count.eq(&5) {
            log::info!("Enabling interupt");
            adxl345.enable_interupt_activity(1_000.0)?;
            match int_pin.get_level() {
                Level::High => log::info!("Interrupt pin is high"),
                Level::Low => log::info!("Interrupt pin is low"),
            }
            // This clears the ADDRESS_INT_SOURCE register
            adxl345.get_interrupt_source()?;
            log::info!("Putting into light sleep");
            FreeRtos::delay_ms(100);
            LightSleep::new()?
                .wakeup_on_gpio(&int_pin, !int_pin.get_level())?
                .enter()?;
            log::info!("Woke up from light sleep");
            let int_source = adxl345.get_interrupt_source()?;
            log::info!("Interrupt source: {:?}", int_source);
            count = 0;
            FreeRtos::delay_ms(1_000);
        }
    }
}
