#![no_std]
#![no_main]
#![feature(panic_info_message, never_type)]
#![allow(unused)]

use core::{alloc::Layout, fmt::Write};

use log::{error, info};

extern crate alloc;

mod arch;
mod common;
mod drivers;
mod kernel;
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

pub fn num_cpus() -> usize {
    todo!()
}
pub fn cpu_id() -> usize {
    todo!()
}

pub fn alloc(layout: Layout) -> Option<*mut u8> {
    todo!()
}
pub fn free(ptr: *mut u8, layout: Layout) {
    todo!()
}
