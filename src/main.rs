#![no_std]
#![no_main]
#![feature(panic_info_message)]

use core::arch::asm;

use log::error;

mod arch;
mod common;
mod drivers;
mod macros;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    match (info.message(), info.location()) {
        (Some(message), Some(location)) => {
            error!(
                "Panic at {}:{}: {}",
                location.file(),
                location.line(),
                message
            );
        }
        (Some(message), None) => {
            error!("Panic: {}", message);
        }
        (None, Some(location)) => {
            error!(
                "Panic at {}:{}: no message provided",
                location.file(),
                location.line()
            );
        }
        (None, None) => {
            error!("Panic: no message provided");
        }
    }

    #[inline(always)]
    fn kalm() -> ! {
        loop {
            unsafe {
                asm!("wfe");
            }
        }
    }
    kalm()
}
