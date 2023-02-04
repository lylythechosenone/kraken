use core::fmt::Debug;

use crate::drivers::MmioDevice;

use super::Serial;

/// Some data, paired with some errors
pub struct Error {
    err: u8,
    data: u8,
}
impl Error {
    /// A framing error occurs when a stop bit is not read
    pub fn framing_err(&self) -> bool {
        self.err & 1 > 0
    }

    /// A parity error occurs when a parity check fails
    pub fn parity_err(&self) -> bool {
        self.err & 2 > 0
    }

    /// A break error occurs when it is detected that the other end has shut down
    pub fn break_err(&self) -> bool {
        self.err & 4 > 0
    }

    /// An overrun error occurs when the FIFO buffer is full and data is received
    pub fn overrun_err(&self) -> bool {
        self.err & 8 > 0
    }

    /// The data read
    pub fn data(&self) -> u8 {
        self.data
    }
}
impl Debug for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Errors")
            .field("framing", &self.framing_err())
            .field("parity", &self.parity_err())
            .field("break", &self.break_err())
            .field("overrun", &self.overrun_err())
            .field("data", &self.data())
            .finish()
    }
}

#[allow(unused)]
pub enum Parity {
    None,
    Odd,
    Even,
}

pub struct Config {
    pub baud_rate: u32,
    pub clock_rate: u32,
    pub parity: Parity,
}

pub struct Pl011 {
    pointer: *mut u8,
}
unsafe impl Send for Pl011 {}
impl MmioDevice for Pl011 {
    fn pointer(&self) -> *mut u8 {
        self.pointer
    }
}
impl Serial for Pl011 {
    type Error = Error;
    type Port = *mut u8;
    type Config = Config;

    unsafe fn new(pointer: Self::Port) -> Self {
        Self { pointer }
    }
    fn init(&mut self, config: Self::Config) -> Result<(), Self::Error> {
        let baud_rate_divisor = config.clock_rate / (16 * config.baud_rate);
        let baud_rate_divisor_fraction = config.clock_rate as u64 * 64 / config.baud_rate as u64;
        self.write_register_16(0x24, baud_rate_divisor as u16);
        self.write_register_8(0x28, (baud_rate_divisor_fraction & 0x3F) as u8); // only 5 bits

        let parity = match config.parity {
            Parity::None => 0b000,
            Parity::Odd => 0b010,
            Parity::Even => 0b110,
        };
        let line_control = parity | 0b01110000;
        self.write_register_8(0x2C, line_control);

        let control = 0b1100001100000001;
        self.write_register_16(0x30, control);

        Ok(()) // `init` never fails
    }

    fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
        self.write_register_8(0x00, byte);
        Ok(()) // `write` never fails
    }
    fn read(&mut self) -> Result<u8, Self::Error> {
        let data = self.read_register_8(0x00);
        let err = self.read_register_8(0x04);
        if err & 0xF > 0 {
            self.write_register_8(0x05, 0);
            return Err(Error { err, data });
        }
        Ok(data)
    }
}
