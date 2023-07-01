use core::arch::asm;

use crate::kernel::memory::{
    address::{PhysAddr, PhysPtr},
    HHDM_START, PHYS_ALLOC,
};

use super::{
    sealed::PageSize, MapError, Mapper, PageFlags, PhysPage, RuntimePageSize, Size1G, Size2M,
    Size4K, TranslateError, VirtPage,
};

pub struct Flush<Size: PageSize>(Option<VirtPage<Size>>);
impl<Size: PageSize> super::CacheFlush for Flush<Size> {
    fn flush(self) {
        if let Some(page) = self.0 {
            unsafe {
                asm!(
                    "dsb st",
                    "tlbi vae1, {}",
                    "dsb sy", "isb",
                    in(reg) page.addr.get() as usize / Size::size(),
                    options(nostack)
                );
            }
        }
    }
}

pub struct PageTable {
    user_l0: [Table; 512],
    kernel_l0: [Table; 512],
}
impl PageTable {
    async fn alloc_tables(hhdm_start: usize) -> Result<PhysPtr<[Table; 512]>, MapError> {
        let phys_alloc = PHYS_ALLOC.get();
        let Some(phys_alloc) = phys_alloc else {
            return Err(MapError::NoPhysAlloc)
        };
        let Some(frame) = phys_alloc.alloc().await else {
            return Err(MapError::OutOfMem)
        };
        let ptr = frame.addr.into_ptr();
        let virt = ptr.to_virt_offset(hhdm_start);
        unsafe {
            core::ptr::write(virt.get(), [Table::new(); 512]);
        }
        Ok(ptr)
    }
    async fn alloc_pages(hhdm_start: usize) -> Result<PhysPtr<[Page; 512]>, MapError> {
        let phys_alloc = PHYS_ALLOC.get();
        let Some(phys_alloc) = phys_alloc else {
            return Err(MapError::NoPhysAlloc)
        };
        let Some(frame) = phys_alloc.alloc().await else {
            return Err(MapError::OutOfMem)
        };
        let ptr = frame.addr.into_ptr();
        let virt = ptr.to_virt_offset(hhdm_start);
        unsafe {
            core::ptr::write(virt.get(), [Page::new(); 512]);
        }
        Ok(ptr)
    }
}
impl Mapper<Size4K> for PageTable {
    type Flush = Flush<Size4K>;

    async fn map(
        &mut self,
        page: VirtPage<Size4K>,
        frame: PhysPage<Size4K>,
        flags: PageFlags,
    ) -> Result<Self::Flush, MapError> {
        let hhdm_start = *HHDM_START.get().unwrap();
        let virt_ptr = page.addr.get() as usize;

        let l0 = if virt_ptr & 1 << 48 > 0 {
            &mut self.kernel_l0
        } else {
            &mut self.user_l0
        };
        let mut l0_desc = &mut l0[virt_ptr >> 39 & 0x1ff];
        if !l0_desc.is_present() {
            l0_desc.set_ptr(Self::alloc_tables(hhdm_start).await?.cast());
            l0_desc.set_present(true);
        }

        let mut virt = l0_desc.get_addr().to_virt_offset(hhdm_start);
        let l1 = unsafe { virt.as_mut() };
        let mut l1_desc = &mut l1[virt_ptr >> 30 & 0x1ff];
        if l1_desc.is_block() {
            return Err(MapError::AlreadyMapped(RuntimePageSize::Size1G));
        }
        let mut l1_desc = unsafe { &mut l1_desc.table };
        if !l1_desc.is_present() {
            l1_desc.set_ptr(Self::alloc_tables(hhdm_start).await?.cast());
            l1_desc.set_present(true);
        }

        let mut virt = l1_desc.get_addr().to_virt_offset(hhdm_start);
        let l2 = unsafe { virt.as_mut() };
        let mut l2_desc = &mut l2[virt_ptr >> 21 & 0x1ff];
        if l2_desc.is_block() {
            return Err(MapError::AlreadyMapped(RuntimePageSize::Size2M));
        }
        let l2_desc = unsafe { &mut l2_desc.table };
        if !l2_desc.is_present() {
            l2_desc.set_ptr(Self::alloc_tables(hhdm_start).await?.cast());
            l2_desc.set_present(true);
        }

        let mut virt = l2_desc.get_addr().to_virt_offset(hhdm_start);
        let l3 = unsafe { virt.as_mut() };
        let mut l3_desc = &mut l3[virt_ptr >> 12 & 0x1ff];
        let l3_desc = unsafe { &mut l3_desc.page };
        if l3_desc.is_present() {
            return Err(MapError::AlreadyMapped(RuntimePageSize::Size4K));
        }

        *l3_desc = Page::from_flags(flags);
        l3_desc.set_addr(frame.addr);
        l3_desc.set_present(true);

        Ok(Flush(None))
    }

    fn unmap(&mut self, page: VirtPage<Size4K>) -> Result<Self::Flush, MapError> {
        let hhdm_start = *HHDM_START.get().unwrap();
        let virt_ptr = page.addr.get() as usize;

        let l0 = if virt_ptr & 1 << 48 > 0 {
            &mut self.kernel_l0
        } else {
            &mut self.user_l0
        };
        let mut l0_desc = &mut l0[virt_ptr >> 39 & 0x1ff];
        if !l0_desc.is_present() {
            return Ok(Flush(None));
        }

        let mut virt = l0_desc.get_addr().to_virt_offset(hhdm_start);
        let l1 = unsafe { virt.as_mut() };
        let mut l1_desc = &mut l1[virt_ptr >> 30 & 0x1ff];
        if l1_desc.is_block() {
            return Err(MapError::AlreadyMapped(RuntimePageSize::Size1G));
        }
        let mut l1_desc = unsafe { &mut l1_desc.table };
        if !l1_desc.is_present() {
            return Ok(Flush(None));
        }

        let mut virt = l1_desc.get_addr().to_virt_offset(hhdm_start);
        let l2 = unsafe { virt.as_mut() };
        let mut l2_desc = &mut l2[virt_ptr >> 21 & 0x1ff];
        if l2_desc.is_block() {
            return Err(MapError::AlreadyMapped(RuntimePageSize::Size2M));
        }
        let l2_desc = unsafe { &mut l2_desc.table };
        if !l2_desc.is_present() {
            return Ok(Flush(None));
        }

        let mut virt = l2_desc.get_addr().to_virt_offset(hhdm_start);
        let l3 = unsafe { virt.as_mut() };
        let mut l3_desc = &mut l3[virt_ptr >> 12 & 0x1ff];
        let l3_desc = unsafe { &mut l3_desc.page };
        if !l3_desc.is_present() {
            return Ok(Flush(None));
        }

        l3_desc.set_present(false);

        Ok(Flush(Some(page)))
    }

    fn translate(
        &mut self,
        page: VirtPage<Size4K>,
    ) -> Result<(PhysPage<Size4K>, PageFlags), TranslateError> {
        let hhdm_start = *HHDM_START.get().unwrap();
        let virt_ptr = page.addr.get() as usize;

        let l0 = if virt_ptr & 1 << 48 > 0 {
            &mut self.kernel_l0
        } else {
            &mut self.user_l0
        };
        let mut l0_desc = &mut l0[virt_ptr >> 39 & 0x1ff];
        if !l0_desc.is_present() {
            return Err(TranslateError::NotPresent);
        }

        let mut virt = l0_desc.get_addr().to_virt_offset(hhdm_start);
        let l1 = unsafe { virt.as_mut() };
        let mut l1_desc = &mut l1[virt_ptr >> 30 & 0x1ff];
        if l1_desc.is_block() {
            return Err(TranslateError::SizeMismatch(RuntimePageSize::Size1G));
        }
        let mut l1_desc = unsafe { &mut l1_desc.table };
        if !l1_desc.is_present() {
            return Err(TranslateError::NotPresent);
        }

        let mut virt = l1_desc.get_addr().to_virt_offset(hhdm_start);
        let l2 = unsafe { virt.as_mut() };
        let mut l2_desc = &mut l2[virt_ptr >> 21 & 0x1ff];
        if l2_desc.is_block() {
            return Err(TranslateError::SizeMismatch(RuntimePageSize::Size2M));
        }
        let l2_desc = unsafe { &mut l2_desc.table };
        if !l2_desc.is_present() {
            return Err(TranslateError::NotPresent);
        }

        let mut virt = l2_desc.get_addr().to_virt_offset(hhdm_start);
        let l3 = unsafe { virt.as_mut() };
        let mut l3_desc = &mut l3[virt_ptr >> 12 & 0x1ff];
        let l3_desc = unsafe { &mut l3_desc.page };
        if !l3_desc.is_present() {
            return Err(TranslateError::NotPresent);
        }

        let addr = l3_desc.get_addr();
        let page = PhysPage::for_addr(addr);

        Ok((page, l3_desc.get_flags()))
    }
}
impl Mapper<Size2M> for PageTable {
    type Flush = Flush<Size2M>;

    async fn map(
        &mut self,
        page: VirtPage<Size2M>,
        frame: PhysPage<Size2M>,
        flags: PageFlags,
    ) -> Result<Self::Flush, MapError> {
        let hhdm_start = *HHDM_START.get().unwrap();
        let virt_ptr = page.addr.get() as usize;

        let l0 = if virt_ptr & 1 << 48 > 0 {
            &mut self.kernel_l0
        } else {
            &mut self.user_l0
        };
        let mut l0_desc = &mut l0[virt_ptr >> 39 & 0x1ff];
        if !l0_desc.is_present() {
            l0_desc.set_ptr(Self::alloc_tables(hhdm_start).await?.cast());
            l0_desc.set_present(true);
        }

        let mut virt = l0_desc.get_addr().to_virt_offset(hhdm_start);
        let l1 = unsafe { virt.as_mut() };
        let mut l1_desc = &mut l1[virt_ptr >> 30 & 0x1ff];
        if l1_desc.is_block() {
            return Err(MapError::AlreadyMapped(RuntimePageSize::Size1G));
        }
        let mut l1_desc = unsafe { &mut l1_desc.table };
        if !l1_desc.is_present() {
            l1_desc.set_ptr(Self::alloc_tables(hhdm_start).await?.cast());
            l1_desc.set_present(true);
        }

        let mut virt = l1_desc.get_addr().to_virt_offset(hhdm_start);
        let l2 = unsafe { virt.as_mut() };
        let l2_desc = &mut l2[virt_ptr >> 21 & 0x1ff];
        if !l2_desc.is_block() && l2_desc.is_present() {
            return Err(MapError::AlreadyMapped(RuntimePageSize::Size4K));
        }
        let l2_desc = unsafe { &mut l2_desc.block };
        if l2_desc.is_present() {
            return Err(MapError::AlreadyMapped(RuntimePageSize::Size2M));
        }

        *l2_desc = Block::from_flags(flags);
        l2_desc.set_addr(frame.addr, 2);
        l2_desc.set_present(true);

        Ok(Flush(None))
    }

    fn unmap(&mut self, page: VirtPage<Size2M>) -> Result<Self::Flush, MapError> {
        let hhdm_start = *HHDM_START.get().unwrap();
        let virt_ptr = page.addr.get() as usize;

        let l0 = if virt_ptr & 1 << 48 > 0 {
            &mut self.kernel_l0
        } else {
            &mut self.user_l0
        };
        let mut l0_desc = &mut l0[virt_ptr >> 39 & 0x1ff];
        if !l0_desc.is_present() {
            return Ok(Flush(None));
        }

        let mut virt = l0_desc.get_addr().to_virt_offset(hhdm_start);
        let l1 = unsafe { virt.as_mut() };
        let mut l1_desc = &mut l1[virt_ptr >> 30 & 0x1ff];
        if l1_desc.is_block() {
            return Err(MapError::AlreadyMapped(RuntimePageSize::Size1G));
        }
        let mut l1_desc = unsafe { &mut l1_desc.table };
        if !l1_desc.is_present() {
            return Ok(Flush(None));
        }

        let mut virt = l1_desc.get_addr().to_virt_offset(hhdm_start);
        let l2 = unsafe { virt.as_mut() };
        let l2_desc = &mut l2[virt_ptr >> 21 & 0x1ff];
        if !l2_desc.is_block() {
            return Err(MapError::AlreadyMapped(RuntimePageSize::Size4K));
        }
        let l2_desc = unsafe { &mut l2_desc.block };
        if !l2_desc.is_present() {
            return Ok(Flush(None));
        }

        l2_desc.set_present(false);

        Ok(Flush(Some(page)))
    }

    fn translate(
        &mut self,
        page: VirtPage<Size2M>,
    ) -> Result<(PhysPage<Size2M>, PageFlags), TranslateError> {
        let hhdm_start = *HHDM_START.get().unwrap();
        let virt_ptr = page.addr.get() as usize;

        let l0 = if virt_ptr & 1 << 48 > 0 {
            &mut self.kernel_l0
        } else {
            &mut self.user_l0
        };
        let mut l0_desc = &mut l0[virt_ptr >> 39 & 0x1ff];
        if !l0_desc.is_present() {
            return Err(TranslateError::NotPresent);
        }

        let mut virt = l0_desc.get_addr().to_virt_offset(hhdm_start);
        let l1 = unsafe { virt.as_mut() };
        let mut l1_desc = &mut l1[virt_ptr >> 30 & 0x1ff];
        if l1_desc.is_block() {
            return Err(TranslateError::SizeMismatch(RuntimePageSize::Size1G));
        }
        let mut l1_desc = unsafe { &mut l1_desc.table };
        if !l1_desc.is_present() {
            return Err(TranslateError::NotPresent);
        }

        let mut virt = l1_desc.get_addr().to_virt_offset(hhdm_start);
        let l2 = unsafe { virt.as_mut() };
        let l2_desc = &mut l2[virt_ptr >> 21 & 0x1ff];
        if !l2_desc.is_block() {
            return Err(TranslateError::SizeMismatch(RuntimePageSize::Size2M));
        }
        let l2_desc = unsafe { &mut l2_desc.block };
        if !l2_desc.is_present() {
            return Err(TranslateError::NotPresent);
        }

        Ok((PhysPage::for_addr(l2_desc.get_addr(2)), l2_desc.get_flags()))
    }
}
impl Mapper<Size1G> for PageTable {
    type Flush = Flush<Size1G>;

    async fn map(
        &mut self,
        page: VirtPage<Size1G>,
        frame: PhysPage<Size1G>,
        flags: PageFlags,
    ) -> Result<Self::Flush, MapError> {
        let hhdm_start = *HHDM_START.get().unwrap();
        let virt_ptr = page.addr.get() as usize;

        let l0 = if virt_ptr & 1 << 48 > 0 {
            &mut self.kernel_l0
        } else {
            &mut self.user_l0
        };
        let mut l0_desc = &mut l0[virt_ptr >> 39 & 0x1ff];
        if !l0_desc.is_present() {
            l0_desc.set_ptr(Self::alloc_tables(hhdm_start).await?.cast());
            l0_desc.set_present(true);
        }

        let mut virt = l0_desc.get_addr().to_virt_offset(hhdm_start);
        let l1 = unsafe { virt.as_mut() };
        let mut l1_desc = &mut l1[virt_ptr >> 30 & 0x1ff];
        if !l1_desc.is_block() && l1_desc.is_present() {
            let l1_desc = unsafe { &mut l1_desc.table };
            let mut virt = l1_desc.get_addr().to_virt_offset(hhdm_start);
            let l2 = unsafe { virt.as_mut() };
            if l2[virt_ptr >> 21 & 0x1ff].is_block() {
                return Err(MapError::AlreadyMapped(RuntimePageSize::Size2M));
            } else {
                return Err(MapError::AlreadyMapped(RuntimePageSize::Size4K));
            }
        }
        let mut l1_desc = unsafe { &mut l1_desc.block };
        if l1_desc.is_present() {
            return Err(MapError::AlreadyMapped(RuntimePageSize::Size1G));
        }

        *l1_desc = Block::from_flags(flags);
        l1_desc.set_addr(frame.addr, 1);
        l1_desc.set_present(true);

        Ok(Flush(None))
    }

    fn unmap(&mut self, page: VirtPage<Size1G>) -> Result<Self::Flush, MapError> {
        let hhdm_start = *HHDM_START.get().unwrap();
        let virt_ptr = page.addr.get() as usize;

        let l0 = if virt_ptr & 1 << 48 > 0 {
            &mut self.kernel_l0
        } else {
            &mut self.user_l0
        };
        let mut l0_desc = &mut l0[virt_ptr >> 39 & 0x1ff];
        if !l0_desc.is_present() {
            return Ok(Flush(None));
        }

        let mut virt = l0_desc.get_addr().to_virt_offset(hhdm_start);
        let l1 = unsafe { virt.as_mut() };
        let mut l1_desc = &mut l1[virt_ptr >> 30 & 0x1ff];
        if !l1_desc.is_block() && l1_desc.is_present() {
            let l1_desc = unsafe { &mut l1_desc.table };
            let mut virt = l1_desc.get_addr().to_virt_offset(hhdm_start);
            let l2 = unsafe { virt.as_mut() };
            if l2[virt_ptr >> 21 & 0x1ff].is_block() {
                return Err(MapError::AlreadyMapped(RuntimePageSize::Size2M));
            } else {
                return Err(MapError::AlreadyMapped(RuntimePageSize::Size4K));
            }
        }
        let mut l1_desc = unsafe { &mut l1_desc.block };
        if !l1_desc.is_present() {
            return Ok(Flush(None));
        }

        l1_desc.set_present(false);

        Ok(Flush(Some(page)))
    }

    fn translate(
        &mut self,
        page: VirtPage<Size1G>,
    ) -> Result<(PhysPage<Size1G>, PageFlags), TranslateError> {
        let hhdm_start = *HHDM_START.get().unwrap();
        let virt_ptr = page.addr.get() as usize;

        let l0 = if virt_ptr & 1 << 48 > 0 {
            &mut self.kernel_l0
        } else {
            &mut self.user_l0
        };
        let mut l0_desc = &mut l0[virt_ptr >> 39 & 0x1ff];
        if !l0_desc.is_present() {
            return Err(TranslateError::NotPresent);
        }

        let mut virt = l0_desc.get_addr().to_virt_offset(hhdm_start);
        let l1 = unsafe { virt.as_mut() };
        let mut l1_desc = &mut l1[virt_ptr >> 30 & 0x1ff];
        if !l1_desc.is_block() && l1_desc.is_present() {
            let l1_desc = unsafe { &mut l1_desc.table };
            let mut virt = l1_desc.get_addr().to_virt_offset(hhdm_start);
            let l2 = unsafe { virt.as_mut() };
            if l2[virt_ptr >> 21 & 0x1ff].is_block() {
                return Err(TranslateError::SizeMismatch(RuntimePageSize::Size2M));
            } else {
                return Err(TranslateError::SizeMismatch(RuntimePageSize::Size4K));
            }
        }
        let mut l1_desc = unsafe { &mut l1_desc.block };
        if !l1_desc.is_present() {
            return Err(TranslateError::NotPresent);
        }

        Ok((PhysPage::for_addr(l1_desc.get_addr(1)), l1_desc.get_flags()))
    }
}

#[derive(Copy, Clone)]
union Entry {
    table: Table,
    block: Block,
    page: Page,
}
impl Entry {
    pub fn is_block(&self) -> bool {
        (unsafe { core::mem::transmute::<_, u64>(*self) }) & 0b10 > 0
    }
    pub fn is_present(&self) -> bool {
        (unsafe { core::mem::transmute::<_, u64>(*self) }) & 1 > 0
    }
}

#[derive(Copy, Clone)]
struct Table {
    data: u64,
}
impl Table {
    const fn new() -> Self {
        Self { data: 0b10 }
    }
    const fn kernel_only() -> Self {
        Self {
            data: 0b11 | 1 << 61,
        }
    }

    fn set_ptr(&mut self, ptr: PhysPtr<[Entry; 512]>) {
        self.data |= ptr.get() as u64 & 0x0000_ffff_ffff_f000;
    }
    fn get_addr(&self) -> PhysPtr<[Entry; 512]> {
        PhysPtr::new((self.data & 0x0000_ffff_ffff_f000) as usize)
    }
    const fn is_present(&self) -> bool {
        self.data & 1 == 1
    }
    fn set_present(&mut self, val: bool) {
        if val {
            self.data |= 1;
        } else {
            self.data &= !1;
        }
    }
}

#[derive(Copy, Clone)]
struct Block {
    data: u64,
}
impl Block {
    const fn new() -> Self {
        Self { data: 0 }
    }
    const fn from_flags(flags: PageFlags) -> Self {
        let mut page = Self::new();
        if flags.contains(PageFlags::USER_ACCESS) {
            page.data |= 1 << 6;
        }
        if !flags.contains(PageFlags::WRITE) {
            page.data |= 1 << 7;
        }
        if !flags.contains(PageFlags::USER_EXEC) {
            page.data |= 1 << 54;
        }
        if !flags.contains(PageFlags::KERNEL_EXEC) {
            page.data |= 1 << 53;
        }
        page
    }

    fn set_addr(&mut self, ptr: PhysAddr, level: usize) {
        match level {
            1 => self.data |= ptr.get() as u64 & 0x0000_ffff_8000_0000,
            2 => self.data |= ptr.get() as u64 & 0x0000_ffff_ffc0_0000,
            _ => panic!("Invalid level"),
        }
    }
    fn get_addr(&mut self, level: usize) -> PhysAddr {
        match level {
            1 => PhysAddr::new((self.data & 0x0000_ffff_8000_0000) as usize),
            2 => PhysAddr::new((self.data & 0x0000_ffff_ffc0_0000) as usize),
            _ => panic!("Invalid level"),
        }
    }
    fn set_present(&mut self, val: bool) {
        if val {
            self.data |= 1;
        } else {
            self.data &= !1;
        }
    }
    const fn is_present(&self) -> bool {
        self.data & 1 == 1
    }

    fn get_flags(&self) -> PageFlags {
        let mut flags = PageFlags::empty();
        if self.data & (1 << 6) > 0 {
            flags.insert(PageFlags::USER_ACCESS);
        }
        if self.data & (1 << 7) == 0 {
            flags.insert(PageFlags::WRITE);
        }
        if self.data & (1 << 54) == 0 {
            flags.insert(PageFlags::USER_EXEC);
        }
        if self.data & (1 << 53) == 0 {
            flags.insert(PageFlags::KERNEL_EXEC);
        }
        if self.data & (1 << 51) > 0 {
            flags.insert(PageFlags::DIRTY);
        }
        flags
    }
}

#[derive(Copy, Clone)]
struct Page {
    data: u64,
}
impl Page {
    const fn new() -> Self {
        Self { data: 0b10 }
    }
    const fn from_flags(flags: PageFlags) -> Self {
        let mut page = Self::new();
        if flags.contains(PageFlags::USER_ACCESS) {
            page.data |= 1 << 6;
        }
        if !flags.contains(PageFlags::WRITE) {
            page.data |= 1 << 7;
        }
        if !flags.contains(PageFlags::USER_EXEC) {
            page.data |= 1 << 54;
        }
        if !flags.contains(PageFlags::KERNEL_EXEC) {
            page.data |= 1 << 53;
        }
        page
    }

    fn set_addr(&mut self, ptr: PhysAddr) {
        self.data |= ptr.get() as u64 & 0x0000_ffff_ffff_f000;
    }
    fn get_addr(&mut self) -> PhysAddr {
        PhysAddr::new((self.data & 0x0000_ffff_ffff_f000) as usize)
    }
    fn set_present(&mut self, val: bool) {
        if val {
            self.data |= 1;
        } else {
            self.data &= !1;
        }
    }
    const fn is_present(&self) -> bool {
        self.data & 1 == 1
    }

    fn get_flags(&self) -> PageFlags {
        let mut flags = PageFlags::empty();
        if self.data & (1 << 6) > 0 {
            flags.insert(PageFlags::USER_ACCESS);
        }
        if self.data & (1 << 7) == 0 {
            flags.insert(PageFlags::WRITE);
        }
        if self.data & (1 << 54) == 0 {
            flags.insert(PageFlags::USER_EXEC);
        }
        if self.data & (1 << 53) == 0 {
            flags.insert(PageFlags::KERNEL_EXEC);
        }
        if self.data & (1 << 51) > 0 {
            flags.insert(PageFlags::DIRTY);
        }
        flags
    }
}
