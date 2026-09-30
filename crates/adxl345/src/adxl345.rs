use anyhow::{Result, bail};
use esp_idf_svc::{
    hal::{delay::TickType, i2c::I2cDriver},
    sys::TickType_t,
};

/// The address of the device ID register
const ADDRESS_DEVICE_ID: u8 = 0x00;
/// The bandwidth rate register address
const ADDRESS_BW_RATE: u8 = 0x2C;
/// The power control register address
const ADDRESS_POWER_CTL: u8 = 0x2D;
/// The address of the I2C device
const ADDRESS_I2C: u8 = 0x53;
/// The address of the data format register
const ADDRESS_DATA_FORMAT: u8 = 0x31;
/// The address of the DATA X 0 register
const ADDRESS_DATAX0: u8 = 0x32;
/// The address of the threshold tap register
const ADDRESS_THRESH_TAP: u8 = 0x1D;
/// The address of the interrupt duration register
const ADDRESS_DUR: u8 = 0x21;
/// The address of the threshold activity register
const ADDRESS_THRESH_ACT: u8 = 0x24;
/// The address of the activity/inactivity control register
const ADDRESS_ACT_INACT_CTL: u8 = 0x27;
/// The address for enabling TAP_X/Y/Z
const ADDRESS_AP_AXES: u8 = 0x2A;
/// The address for enabling interrupts
const ADDRESS_INT_ENABLE: u8 = 0x2E;
/// The address for reading interrupt sources
const ADDRESS_INT_SOURCE: u8 = 0x30;

/// The full resolution bit in the data format register
const DATA_FORMAT_FULL_RES: u8 = 0b1000;
/// The value for setting the power control to measure mode
const POWER_CTL_MEASURE: u8 = 0b1000;
/// The value for setting the power control to standby mode
const POWER_CTL_STANDBY: u8 = 0b0000;
/// The device ID
const DEVICE_ID: u8 = 0xE5;
/// The bit for enabling ACTIVITY in the INT_ENABLE register
const INT_ENABLE_ACTIVITY: u8 = 0b0001_0000;
/// The bit for enabling ACT_X in the ACT_INACT_CTL register
const ACT_INACT_CTL_ACT_X_ENABLED: u8 = 0b0100_0000;
/// The bit for enabling ACT_Y in the ACT_INACT_CTL register
const ACT_INACT_CTL_ACT_Y_ENABLED: u8 = 0b0010_0000;
/// The bit for enabling ACT_Z in the ACT_INACT_CTL register
const ACT_INACT_CTL_ACT_Z_ENABLED: u8 = 0b0001_0000;
/// The bit for enabling AC in the ACT_INACT_CTL ACT register
const ACT_INACT_CTL_ACT_AC_ENABLED: u8 = 0b1000_0000;

/// The bit for whether single tap is in the INT_SOURCE register
const INT_SOURCE_SINGLE_TAP: u8 = 0b0100_0000;
/// The bit for whether double tap is in the INT_SOURCE register
const INT_SOURCE_DOUBLE_TAP: u8 = 0b0010_0000;
/// The bit for whether activity is in the INT_SOURCE register
const INT_SOURCE_ACTIVITY: u8 = 0b0001_0000;
/// The bit for whether inactivity is in the INT_SOURCE register
const INT_SOURCE_INACTIVITY: u8 = 0b0000_1000;

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

        if adxl.read_command(ADDRESS_DEVICE_ID)? != DEVICE_ID {
            bail!("Device ID does not match");
        }

        adxl.set_measurement_range(MeasurementRange::G4)?;
        adxl.turn_on_measure_mode()?;

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
    ///
    /// # Returns
    ///
    /// (x, y, x) measurements in mg
    pub fn read_raw(&mut self) -> Result<(i16, i16, i16)> {
        let mut buffer = [0u8; 6];
        self.i2c_driver
            .write_read(ADDRESS_I2C, &[ADDRESS_DATAX0], &mut buffer, self.timeout)?;

        let x = i16::from_le_bytes([buffer[0], buffer[1]]);
        let y = i16::from_le_bytes([buffer[2], buffer[3]]);
        let z = i16::from_le_bytes([buffer[4], buffer[5]]);

        Ok((x * 4, y * 4, z * 4))
    }

    /// Enable the activity interrupt with the given threshold. The threshold
    /// should be given in mg ranging from 0 to 16,000
    pub fn enable_interupt_activity(&mut self, threshold: f32) -> Result<()> {
        if !(0.0..=16_000.0).contains(&threshold) {
            bail!("Threshold out or range: {threshold}");
        }

        log::info!("Setting threshold to {threshold}mg");
        let threshold = (threshold / 62.5).round().clamp(0.0, 255.0) as u8;

        self.write_command(ADDRESS_INT_ENABLE, 0)?;
        self.write_command(ADDRESS_THRESH_ACT, threshold)?;
        self.write_command(
            ADDRESS_ACT_INACT_CTL,
            ACT_INACT_CTL_ACT_AC_ENABLED
                | ACT_INACT_CTL_ACT_X_ENABLED
                | ACT_INACT_CTL_ACT_Y_ENABLED
                | ACT_INACT_CTL_ACT_Z_ENABLED,
        )?;
        self.write_command(ADDRESS_INT_ENABLE, INT_ENABLE_ACTIVITY)?;

        // This clears the ADDRESS_INT_SOURCE register
        self.get_interrupt_source().ok();

        Ok(())
    }

    /// Read and decode the [`ADDRESS_INT_SOURCE`] register.
    pub fn get_interrupt_source(&mut self) -> Result<InterruptSource> {
        let int_source = self.read_command(ADDRESS_INT_SOURCE)?;

        if int_source & INT_SOURCE_SINGLE_TAP == INT_SOURCE_SINGLE_TAP {
            return Ok(InterruptSource::SingleTap);
        } else if int_source & INT_SOURCE_DOUBLE_TAP == INT_SOURCE_DOUBLE_TAP {
            return Ok(InterruptSource::DoubleTap);
        } else if int_source & INT_SOURCE_ACTIVITY == INT_SOURCE_ACTIVITY {
            return Ok(InterruptSource::Activity);
        } else if int_source & INT_SOURCE_INACTIVITY == INT_SOURCE_INACTIVITY {
            return Ok(InterruptSource::Inactivity);
        } else {
            bail!("Unknown interrupt source: {int_source:#08b}");
        }
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

/// A type of interrupt source for [`ADXL345`].
#[derive(Debug)]
pub enum InterruptSource {
    SingleTap,
    DoubleTap,
    Activity,
    Inactivity,
    DataReady,
}
