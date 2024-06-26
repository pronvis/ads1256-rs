//! A platform agnostic driver to interface with the  ADS1256 analog-digital converter
//!
//! This driver was built using [`embedded-hal`] traits.
//!
//!
//!
//! # Example
//!   TODO:
//!
//!
//! [datasheet] : http://www.ti.com/lit/ds/symlink/ads1256.pdf

#![no_std]

#[deny(missing_docs)]
extern crate embedded_hal as hal;

//use hal::blocking::spi::{Transfer, Write};
use hal::blocking::delay::DelayUs;

/// ADC reference voltage in volts
const REF_VOLTS: f64 = 2.5;

/// Read / write-able registers
///
/// Table 23 page 30 of specification.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
pub enum Register {
    /// STATUS REGISTER
    STATUS = 0x00,
    /// Input Multiplexer Control Register
    MUX = 0x01,
    /// A/D Control Register
    ADCON = 0x02,
    /// A/D Data Rate
    DRATE = 0x03,
    /// GPIO Control Registe
    IO = 0x04,
    /// Offset Calibration Byte 0, least significant byte
    OFC0 = 0x05,
    /// Offset Calibration Byte 1
    OFC1 = 0x06,
    /// Offset Calibration Byte 2, most significant byte
    OFC2 = 0x07,
    /// Full−scale Calibration Byte 0, least significant byte
    FSC0 = 0x08,
    /// Full−scale Calibration Byte 1
    FSC1 = 0x09,
    /// Full−scale Calibration Byte 2, most significant byte
    FSC2 = 0x0A,
}

impl Register {
    fn addr(self) -> u8 {
        self as u8
    }
}

/// The commands control the operation of the ADS1256.
/// CS must stay low during the entire command sequence.
/// Table 24 page 34 of specification.
pub enum Command {
    /// Completes SYNC and Exits Standby Mode
    WAKEUP = 0b_0000_0000, // or 0b_1111_1111
    /// Read data
    RDATA = 0b_0000_0001,
    /// Read Data Continuously
    RDATAC = 0b_0000_0011,
    /// Stop Read Data Continuously
    SDATAC = 0b_0000_1111,
    /// Read registers starting at an address
    RREG = 0b_0001_0000,
    /// Write registers starting at an address
    WREG = 0b_0101_0000,
    /// Offset and Gain Self-Calibration
    SELFCAL = 0b_1111_0000,
    /// Offset Self-Calibration
    SELFOCAL = 0b_1111_0001,
    /// Gain Self-Calibration
    SELFGCAL = 0b_1111_0010,
    /// System Offset Calibration
    SYSOCAL = 0b_1111_0011,
    /// System Gain Calibration
    SYSGCAL = 0b_1111_0100,
    /// Synchronize the A/D Conversion
    SYNC = 0b_1111_1100,
    /// Begin Standby Mode
    STANDBY = 0b_1111_1101,
    /// Reset to Power-Up Values
    RESET = 0b_1111_1110,
}

impl Command {
    fn bits(self) -> u8 {
        self as u8
    }
}

///Programmable Gain Amplifier (pga) ads1256 datasheet, p. 16
#[derive(Debug, Copy, Clone)]
pub enum PGA {
    Gain1 = 0b000,
    Gain2 = 0b001,
    Gain4 = 0b010,
    Gain8 = 0b011,
    Gain16 = 0b100,
    Gain32 = 0b101,
    Gain64 = 0b110,
}

impl Default for PGA {
    fn default() -> Self {
        PGA::Gain1
    }
}

impl PGA {
    pub fn bits(self) -> u8 {
        self as u8
    }

    pub fn val(self) -> u8 {
        1 << self as u8
    }
}

//Sampling rate
#[derive(Debug, Copy, Clone)]
pub enum SamplingRate {
    Sps30000 = 0b1111_0000,
    Sps15000 = 0b1110_0000,
    Sps7500 = 0b1101_0000,
    Sps3750 = 0b1100_0000,
    Sps2000 = 0b1011_0000,
    Sps1000 = 0b1010_0001,
    Sps500 = 0b1001_0010,
    Sps100 = 0b1000_0010,
    Sps60 = 0b0111_0010,
    Sps50 = 0b0110_0011,
    Sps30 = 0b0101_0011,
    Sps25 = 0b0100_0011,
    Sps15 = 0b0011_0011,
    Sps10 = 0b0010_0011,
    Sps5 = 0b0001_0011,
    Sps2_5 = 0b0000_0011,
}

impl SamplingRate {
    fn bits(self) -> u8 {
        self as u8
    }
}

impl Default for SamplingRate {
    fn default() -> Self {
        SamplingRate::Sps1000
    }
}

//Channel
#[derive(Debug, Copy, Clone)]
pub enum Channel {
    AIN0 = 0,
    AIN1 = 1,
    AIN2 = 2,
    AIN3 = 3,
    AIN4 = 4,
    AIN5 = 5,
    AIN6 = 6,
    AIN7 = 7,
    AINCOM = 8,
}

impl Channel {
    fn bits(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Config {
    pub sampling_rate: SamplingRate,
    pub gain: PGA,
}

impl Config {
    pub fn new(sampling_rate: SamplingRate, gain: PGA) -> Self {
        Config {
            sampling_rate,
            gain,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            sampling_rate: SamplingRate::Sps1000,
            gain: PGA::Gain1,
        }
    }
}

//ADS1256 driver
#[derive(Debug, Default)]
pub struct ADS1256<SPI, CS, RST, DRDY, D> {
    ///Dedicated GPIO pin  that is used to select ADS1256 chip on the SPI bus
    cs_pin: CS,
    ///Dedicated GPIO pin to reset the ADS1256
    reset_pin: RST,
    ///Dedicated GPIO pin to indicate that conversion is ready
    data_ready_pin: DRDY,
    spi: SPI,
    delay: D,
    config: Config,
}

impl<SPI, CS, RST, DRDY, D, E> ADS1256<SPI, CS, RST, DRDY, D>
where
    SPI: hal::blocking::spi::Transfer<u8, Error = E> + hal::blocking::spi::Write<u8, Error = E>,
    CS: hal::digital::v2::OutputPin,
    RST: hal::digital::v2::OutputPin,
    DRDY: hal::digital::v2::InputPin,
    D: DelayUs<u8>,
{
    /// Creates a new driver from a SPI
    pub fn new(
        spi: SPI,
        cs_pin: CS,
        reset_pin: RST,
        data_ready_pin: DRDY,
        delay: D,
    ) -> Result<Self, E> {
        let mut ads1256 = ADS1256 {
            spi,
            cs_pin,
            reset_pin,
            data_ready_pin,
            delay,
            config: Config::default(),
        };

        //stop read data continuously
        ads1256.wait_for_ready();
        ads1256.send_command(Command::SDATAC)?;
        ads1256.delay.delay_us(10);
        Ok(ads1256)
    }

    pub fn set_config(&mut self, config: &Config) -> Result<(), E> {
        self.config = *config;
        self.init()?;
        Ok(())
    }

    pub fn init(&mut self) -> Result<(), E> {
        let adcon = self.read_register(Register::ADCON)?;
        //disable clkout and set the gain
        let new_adcon = (adcon & 0x07) | self.config.gain.bits();
        self.write_register(Register::ADCON, new_adcon)?;
        self.write_register(Register::DRATE, self.config.sampling_rate.bits())?;
        self.send_command(Command::SELFCAL)?;
        self.wait_for_ready(); //wait for calibration to complete
        Ok(())
    }

    ///Returns true if conversion data is ready to  transmit to the host
    pub fn wait_for_ready(&self) -> bool {
        self.data_ready_pin.is_low()
    }

    ///Read data from specified register
    pub fn read_register(&mut self, reg: Register) -> Result<u8, E> {
        self.cs_pin.set_low();
        //write
        self.spi
            .write(&[(Command::RREG.bits() | reg.addr()), 0x00])?;
        self.delay.delay_us(10); //t6 delay
                                 //read
        let mut rx_buf = [0];
        self.spi.transfer(&mut rx_buf)?;
        self.delay.delay_us(5); //t11
        self.cs_pin.set_high();
        Ok(rx_buf[0])
    }

    ///Write data to specified register
    pub fn write_register(&mut self, reg: Register, val: u8) -> Result<(), E> {
        self.cs_pin.set_low();

        let mut tx_buf = [(Command::WREG.bits() | reg.addr()), 0x00, val];
        self.spi.transfer(&mut tx_buf)?;
        self.delay.delay_us(5); //t11
        self.cs_pin.set_high();
        Ok(())
    }

    pub fn send_command(&mut self, cmd: Command) -> Result<(), E> {
        self.cs_pin.set_low();
        self.spi.write(&[cmd.bits()])?;
        self.cs_pin.set_high();
        Ok(())
    }

    ///Read 24 bit value from ADS1256. Issue this command after DRDY goes low
    fn read_raw_data(&mut self) -> Result<i32, E> {
        self.cs_pin.set_low();
        self.spi.write(&[Command::RDATA.bits()])?;
        self.delay.delay_us(10); //t6 delay = 50*0.13=6.5us
                                 //receive 3 bytes from spi
        let mut buf = [0u8; 3];
        self.spi.transfer(&mut buf)?;
        self.cs_pin.set_high();

        let mut result: u32 = ((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[2] as u32);
        //sign extension if result is negative
        if (result & 0x800000) != 0 {
            result |= 0xFF000000;
        }
        Ok(result as i32)
    }

    ///Read an ADC channel and returned  24 bit value as i32
    pub fn read_channel(&mut self, ch1: Channel, ch2: Channel) -> Result<i32, E> {
        //wait form data ready pin to be low
        self.wait_for_ready();

        //select channel
        self.write_register(Register::MUX, ch1.bits() << 4 | ch2.bits())?;

        //start conversion
        self.send_command(Command::SYNC)?;
        self.delay.delay_us(5); //t11

        self.send_command(Command::WAKEUP)?;
        self.delay.delay_us(1); //t11

        //read channel data
        let adc_code = self.read_raw_data()?;

        Ok(adc_code)
    }

    pub fn convert_to_volt(&self, code: i32) -> f64 {
        (code as f64) / (0x7FFFFF as f64) * (2.0 * REF_VOLTS) / (self.config.gain.val() as f64)
    }
}
