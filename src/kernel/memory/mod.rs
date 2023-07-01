use core::alloc::GlobalAlloc;

use log::trace;
use mem::vmem::Vmem;
use spin::{Mutex, Once};

use self::{address::PhysAddr, physalloc::PhysAlloc};

pub mod address;
pub mod physalloc;

pub static HHDM_START: Once<usize> = Once::new();
pub static PHYS_ALLOC: Once<PhysAlloc> = Once::new();

#[global_allocator]
pub static DUMMY_ALLOC: DummyAlloc = DummyAlloc;

pub struct DummyAlloc;
unsafe impl GlobalAlloc for DummyAlloc {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        unimplemented!()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        unimplemented!()
    }
}

pub struct Kmem {
    pub physalloc: PhysAlloc,
    pub vmem: Vmem<'static>,
}
