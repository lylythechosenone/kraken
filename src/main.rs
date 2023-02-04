#![no_std]
#![no_main]
#![feature(panic_info_message)]

use core::arch::asm;

use log::{error, info};

mod arch;
mod common;
mod drivers;
mod macros;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    match (info.message(), info.location()) {
        (Some(message), Some(location)) => {
            error!(target: "panic",
                "Panic at {}:{}: {}",
                location.file(),
                location.line(),
                message
            );
        }
        (Some(message), None) => {
            error!(target: "panic", "Panic: {}", message);
        }
        (None, Some(location)) => {
            error!(
                target: "panic",
                "Panic at {}:{}: no message provided",
                location.file(),
                location.line()
            );
        }
        (None, None) => {
            error!(target: "panic", "Panic: no message provided");
        }
    }

    #[inline(always)]
    fn kalm() -> ! {
        arch::util::wait_forever();
    }
    kalm()
}

pub fn main() -> ! {
    info!("Main called!");
    panic!("Got to the end of main");
}
