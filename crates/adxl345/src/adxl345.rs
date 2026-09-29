use anyhow::Result;
use esp_idf_svc::{
    hal::{delay::TickType, i2c::I2cDriver},
    sys::TickType_t,
};

/// The power control register address
const ADDRESS_POWER_CTL: u8 = 0x2D;
/// The bandwidth rate register address
const ADDRESS_BW_RATE: u8 = 0x2C;
/// The address of the data format register
const ADDRESS_DATA_FORMAT: u8 = 0x31;
/// The address of the I2C device
const ADDRESS_I2C: u8 = 0x1D;

pub struct ADXL345<'d> {
    i2c_driver: I2cDriver<'d>,
    timeout: TickType_t,
}

/// The measurement range of the ADXL345.
pub enum MeasurementRange {
    /// ±2g
    G2 = 0b00,
    /// ±4g
    G4 = 0b01,
    /// ±8g
    G8 = 0b10,
    /// ±16g
    G16 = 0b11,
}

impl<'d> ADXL345<'d> {
    pub fn new(i2c_driver: I2cDriver<'d>) -> Result<Self> {
        let timeout = TickType::new_millis(100).ticks();

        Ok(Self { i2c_driver, timeout })
    }

    pub fn set_measurement_range(&mut self, range: MeasurementRange) -> Result<()> {
        let current = self.read_command(ADDRESS_DATA_FORMAT)?;
        let new = current & 0b1111_0000 | range as u8;
        self.write_command(ADDRESS_DATA_FORMAT, new)
    }

    pub fn turn_on_measure_mode(&mut self) -> Result<()> {
        let current = self.read_command(ADDRESS_POWER_CTL)?;
        let new = current | 0b0000_1000;
        self.write_command(ADDRESS_POWER_CTL, new)
    }

    pub fn read_command(&mut self, address: u8) -> Result<u8> {
        let mut buffer = [0u8; 1];
        self.i2c_driver.read(address, &mut buffer, self.timeout)?;
        Ok(buffer[0])
    }

    fn write_command(&mut self, address: u8, data: u8) -> Result<()> {
        self.i2c_driver.write(ADDRESS_I2C, &[address, data], self.timeout)?;
        Ok(())
    }
}
