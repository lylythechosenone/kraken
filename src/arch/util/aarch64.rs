use core::arch::asm;

pub fn wait_forever() -> ! {
    loop {
        unsafe {
            asm!("wfe");
        }
    }
}
