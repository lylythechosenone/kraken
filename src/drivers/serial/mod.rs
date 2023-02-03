pub mod pl011;

use core::fmt::{Debug, Write};

mod sealed {
    /// A serial port. Can be a pointer or an I/O port.
    pub trait SerialPort {}
    impl<T> SerialPort for *mut T {}
    impl SerialPort for u16 {}
}

/// A serial device. Can be MMIO or simple I/O ports.
pub trait Serial {
    type Error: Debug;
    type Port: sealed::SerialPort;
    type Config;

    /// Create a new serial instance of this type.
    unsafe fn new(port: Self::Port) -> Self;
    /// Initialize the serial device. This includes things like configuring the baud rate.
    fn init(&mut self, config: Self::Config) -> Result<(), Self::Error>;

    /// Write a single byte to the serial device.
    fn write(&mut self, byte: u8) -> Result<(), Self::Error>;
    /// Read a single byte from the serial device. Block if no data is available.
    fn read(&mut self) -> Result<u8, Self::Error>;
    /// Check if there is data available to read.
    ///
    /// Only reimplement this if it is supported on your device. By default, it returns `Ok(true)` every time.
    fn read_ready(&self) -> Result<bool, Self::Error> {
        Ok(true)
    }

    /// Write multiple bytes to the serial port.
    ///
    /// Only re-implement this if you can do it more efficiently than calling `write` multiple times.
    fn write_multi(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        for byte in bytes {
            self.write(*byte)?;
        }
        Ok(())
    }
}

#[repr(transparent)]
pub struct SerialWriter<T: Serial>(pub T);
impl<T: Serial> Write for SerialWriter<T> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.0
            .write_multi(s.as_bytes())
            .map_err(|_| core::fmt::Error)
    }
}
