use core::alloc::GlobalAlloc;

use log::trace;
use mem::vmem::Vmem;
use spin::{Mutex, Once};

use self::address::PhysAddr;

pub mod address;
pub mod physalloc;

pub static BITMAP: Mutex<Option<Bitmap>> = Mutex::new(None);
pub static HHDM_START: Once<usize> = Once::new();

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

pub struct Bitmap {
    pub data: &'static mut [u64],
}
impl Bitmap {
    /// Why is this a `*mut ()`? The actual type is an implementation detail.
    ///
    /// # Safety
    /// The pointer must be valid.
    pub unsafe fn from_ptr(ptr: *mut (), size: usize) -> Self {
        Self {
            data: unsafe { core::slice::from_raw_parts_mut(ptr as *mut u64, size) },
        }
    }

    /// Set a range of pages as allocated.
    ///
    /// # Safety
    /// The range must be valid (start cannot be end).
    pub unsafe fn set_range(&mut self, start: usize, end: usize) {
        trace!(
            "Marking {:p} - {:p} as reserved",
            (start * 4096) as *const (),
            (end * 4096) as *const ()
        );
        let start_big = start / 64;
        let end_big = end / 64;
        let start_small = start % 64;
        let end_small = end % 64;
        if start_big == end_big {
            self.data[start_big] |= u64::MAX >> (64 - (end_small - start_small)) << start_small;
        } else {
            self.data[start_big] |= !0 << start_small;
            for i in start_big + 1..end_big {
                self.data[i] = !0;
            }
            self.data[end_big] |= (1 << end_small) - 1;
        }
    }

    /// Set a range of pages as free.
    /// # Safety
    /// The range must be valid (start cannot be end).
    pub unsafe fn clear_range(&mut self, start: usize, end: usize) {
        trace!(
            "Marking {:p} - {:p} as free",
            (start * 4096) as *const (),
            (end * 4096) as *const ()
        );
        let start_big = start / 64;
        let end_big = end / 64;
        let start_small = start % 64;
        let end_small = end % 64;
        if start_big == end_big {
            self.data[start_big] &= !(u64::MAX >> (64 - (end_small - start_small)) << start_small);
        } else {
            self.data[start_big] &= !(!0 << start_small);
            for i in start_big + 1..end_big {
                self.data[i] = 0;
            }
            self.data[end_big] &= !((1 << end_small) - 1);
        }
    }

    pub fn alloc(&mut self) -> Option<PhysAddr> {
        for (i, &x) in self.data.iter().enumerate() {
            if x != !0 {
                let bit = x.trailing_zeros();
                self.data[i] |= 1 << bit;
                return Some(PhysAddr::new((i * 64 + bit as usize) * 4096));
            }
        }
        None
    }
}

pub struct Kmem {
    pub bitmap: &'static mut Bitmap,
    pub vmem: &'static mut Vmem<'static>,
}
