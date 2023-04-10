use core::alloc::Layout;

extern "Rust" {
    pub fn num_cpus() -> usize;
    pub fn cpu_id() -> usize;

    pub fn alloc(layout: Layout) -> Option<*mut u8>;
    pub fn free(ptr: *mut u8, layout: Layout);
}

#[cfg(feature = "userspace")]
mod userspace;
