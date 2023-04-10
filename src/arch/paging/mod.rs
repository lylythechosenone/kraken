use core::marker::PhantomData;

use bitflags::bitflags;

use crate::kernel::memory::address::{PhysAddr, VirtAddr};

use self::sealed::PageSize;

pub mod aarch64;

mod sealed {
    pub trait PageSize {
        fn size() -> usize;
    }
}

pub struct Size4K;
impl PageSize for Size4K {
    fn size() -> usize {
        4096
    }
}
pub struct Size2M;
impl PageSize for Size2M {
    fn size() -> usize {
        2 * 1024 * 1024
    }
}
pub struct Size1G;
impl PageSize for Size1G {
    fn size() -> usize {
        1024 * 1024 * 1024
    }
}

pub enum RuntimePageSize {
    Size4K,
    Size2M,
    Size1G,
}

pub struct PhysPage<Size: PageSize> {
    addr: PhysAddr,
    _phantom: PhantomData<Size>,
}
impl<Size: PageSize> PhysPage<Size> {
    pub fn for_addr(addr: PhysAddr) -> Self {
        Self {
            addr: PhysAddr::new(addr.get() / Size::size() * Size::size()),
            _phantom: PhantomData,
        }
    }
    pub fn addr(&self) -> PhysAddr {
        self.addr
    }
}

pub enum TranslateError {
    NotPresent,
    SizeMismatch(RuntimePageSize),
}

pub enum MapError {
    AlreadyMapped(RuntimePageSize),
    NoBitmap,
    OutOfMem,
}

pub struct VirtPage<Size: PageSize> {
    addr: VirtAddr,
    _phantom: PhantomData<Size>,
}
impl<Size: PageSize> VirtPage<Size> {
    pub fn for_addr(addr: VirtAddr) -> Self {
        Self {
            addr: VirtAddr::new((addr.get() as usize / Size::size() * Size::size()) as *mut _),
            _phantom: PhantomData,
        }
    }
}

bitflags! {
    pub struct PageFlags: u64 {
        const KERNEL_EXEC = 1;
        const USER_EXEC = 1 << 1;
        const WRITE = 1 << 2;
        const USER_ACCESS = 1 << 3;
        const DIRTY = 1 << 4;
    }
}

pub trait Mapper<Size: PageSize> {
    type Flush: CacheFlush;
    fn map(
        &mut self,
        page: VirtPage<Size>,
        frame: PhysPage<Size>,
        flags: PageFlags,
    ) -> Result<Self::Flush, MapError>;
    fn unmap(&mut self, page: VirtPage<Size>) -> Result<Self::Flush, MapError>;
    fn translate(
        &mut self,
        page: VirtPage<Size>,
    ) -> Result<(PhysPage<Size>, PageFlags), TranslateError>;
}

pub trait CacheFlush: Sized {
    fn flush(self);
    // instant drop
    fn ignore(self) {}
}
