use anyhow::{Result, bail};
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
const ADDRESS_I2C: u8 = 0x53;
/// The address of the device ID register
const ADDRESS_DEVICE_ID: u8 = 0x00;
/// The address of the DATA X 0 register
const ADDRESS_DATAX0: u8 = 0x32;

/// The full resolution bit in the data format register
const DATA_FORMAT_FULL_RES: u8 = 0b1000;
/// The value for setting the power control to measure mode
const POWER_CTL_MEASURE: u8 = 0b1000;
/// The value for setting the power control to standby mode
const POWER_CTL_STANDBY: u8 = 0b0000;
/// The device ID
const DEVICE_ID: u8 = 0xE5;

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
    /// Create a new ADXL345 instance with the given I2C driver. This will also
    /// set the measurement range to ±4g and turn on measure mode.
    pub fn new(i2c_driver: I2cDriver<'d>) -> Result<Self> {
        let timeout = TickType::new_millis(100).ticks();
        let mut adxl = Self { i2c_driver, timeout };

        if !adxl.read_command(ADDRESS_DEVICE_ID)?.eq(&DEVICE_ID) {
            bail!("Device ID does not match");
        }

        adxl.set_measurement_range(MeasurementRange::G4)?;
        adxl.turn_on_measure_mode()?;

        // TODO: interupt pins

        Ok(adxl)
    }

    pub fn set_measurement_range(&mut self, range: MeasurementRange) -> Result<()> {
        self.write_command(ADDRESS_DATA_FORMAT, DATA_FORMAT_FULL_RES | range as u8)
    }

    /// Set the power control to measure mode.
    pub fn turn_on_measure_mode(&mut self) -> Result<()> {
        self.write_command(ADDRESS_POWER_CTL, POWER_CTL_MEASURE)
    }

    /// Set the power control to standby mode.
    pub fn turn_on_standby_mode(&mut self) -> Result<()> {
        self.write_command(ADDRESS_POWER_CTL, POWER_CTL_STANDBY)
    }

    /// Read the raw DATAX/DATAY/DATAZ buffers
    pub fn read_raw(&mut self) -> Result<(i16, i16, i16)> {
        let mut buffer = [0u8; 6];
        self.i2c_driver
            .write_read(ADDRESS_I2C, &[ADDRESS_DATAX0], &mut buffer, self.timeout)?;

        let x = i16::from_le_bytes([buffer[0], buffer[1]]);
        let y = i16::from_le_bytes([buffer[2], buffer[3]]);
        let z = i16::from_le_bytes([buffer[4], buffer[5]]);

        Ok((x, y, z))
    }

    /// Read a single byte from the given address.
    fn read_command(&mut self, address: u8) -> Result<u8> {
        let mut buffer = [0u8; 1];
        self.i2c_driver
            .write_read(ADDRESS_I2C, &[address], &mut buffer, self.timeout)?;
        Ok(buffer[0])
    }

    /// Write a single byte to the given address.
    fn write_command(&mut self, address: u8, data: u8) -> Result<()> {
        self.i2c_driver.write(ADDRESS_I2C, &[address, data], self.timeout)?;
        Ok(())
    }
}
