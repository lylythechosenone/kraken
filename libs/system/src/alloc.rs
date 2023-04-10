use core::alloc::Layout;

pub fn alloc(layout: Layout) -> Option<*mut u8> {
    unsafe { crate::backend::alloc(layout) }
}

pub fn free(ptr: *mut u8, layout: Layout) {
    unsafe { crate::backend::free(ptr, layout) }
}
