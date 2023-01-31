#![no_std]
#![no_main]
#![feature(panic_info_message)]

use core::fmt::Write;

use uart_16550::MmioSerialPort;

mod arch;
mod common;
mod macros;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let mut serial = unsafe { MmioSerialPort::new(0x9000000) };
    write!(serial, "Panic: {}", info.message().unwrap()).unwrap();
    #[inline(always)]
    fn kalm() -> ! {
        loop {
            core::hint::spin_loop();
        }
    }
    kalm()
}
