pub mod pl011;

use core::{
    arch::asm,
    fmt::{Debug, Write},
};

use log::Log;
use spin::{Mutex, MutexGuard};

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

struct SerialWriter<'a, 'mutex, T: Serial> {
    serial: &'a mut MutexGuard<'mutex, T>,
}
impl<'a, 'mutex, T: Serial> SerialWriter<'a, 'mutex, T> {
    pub fn new(serial: &'a mut MutexGuard<'mutex, T>) -> Self {
        Self { serial }
    }
}
impl<'a, 'mutex, T: Serial> Write for SerialWriter<'a, 'mutex, T> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.serial
            .write_multi(s.as_bytes())
            .map_err(|_| core::fmt::Error)
    }
}

pub struct SerialLogger<T: Serial + Send> {
    serial: Mutex<T>,
}
impl<T: Serial + Send> SerialLogger<T> {
    pub fn new(serial: T) -> Self {
        Self {
            serial: Mutex::new(serial),
        }
    }
    pub fn set_logger(&'static self) -> Result<(), log::SetLoggerError> {
        log::set_logger(self)
    }
}
impl<T: Serial + Send> Log for SerialLogger<T> {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }
    fn log(&self, record: &log::Record) {
        match record.level() {
            log::Level::Error => {
                let mut serial = self.serial.lock();
                let mut writer = SerialWriter::new(&mut serial);
                writeln!(writer, "[ERROR] {} {}", record.target(), record.args()).unwrap();
            }
            log::Level::Warn => {
                let mut serial = self.serial.lock();
                let mut writer = SerialWriter::new(&mut serial);
                writeln!(writer, "[WARN] {} {}", record.target(), record.args()).unwrap();
            }
            log::Level::Info => {
                let mut serial = self.serial.lock();
                let mut writer = SerialWriter::new(&mut serial);
                writeln!(writer, "[INFO] {} {}", record.target(), record.args()).unwrap();
            }
            log::Level::Debug => {
                let mut serial = self.serial.lock();
                let mut writer = SerialWriter::new(&mut serial);
                writeln!(writer, "[DEBUG] {} {}", record.target(), record.args()).unwrap();
            }
            log::Level::Trace => {
                let mut serial = self.serial.lock();
                let mut writer = SerialWriter::new(&mut serial);
                writeln!(writer, "[TRACE] {} {}", record.target(), record.args()).unwrap();
            }
        }
    }
    fn flush(&self) {}
}
