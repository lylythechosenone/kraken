#![no_std]
#![no_main]
#![feature(panic_info_message)]

mod arch;
mod common;
mod drivers;
mod macros;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    //write!(serial, "Panic: {}", info.message().unwrap()).unwrap();

    #[inline(always)]
    fn kalm() -> ! {
        loop {
            core::hint::spin_loop();
        }
    }
    kalm()
}
