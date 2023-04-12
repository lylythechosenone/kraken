use core::{fmt::Debug, ptr::NonNull};

use crate::{
    arch::paging::{PhysPage, Size4K},
    common::sizes::Size,
    size_of,
};

use super::address::{PhysAddr, Pointer, Virtual};

#[derive(Debug)]
pub struct Node {
    pub next: Option<NonNull<Node>>,
}

pub struct PhysAlloc {
    pub free: Option<NonNull<Node>>,
    pub dirty: Option<NonNull<Node>>,
}
impl PhysAlloc {
    pub fn alloc(&mut self) -> Option<PhysPage<Size4K>> {
        let mut node = if let Some(free) = self.free {
            free
        } else if let Some(dirty) = self.dirty {
            let ptr = dirty.as_ptr().wrapping_add(1);
            unsafe {
                ptr.write_bytes(0, 4096 - size_of!(Node));
            }
            dirty
        } else {
            return None;
        };
        let node_ref = unsafe { node.as_mut() };
        self.free = node_ref.next;
        node_ref.next = None;
        Some(PhysPage::for_addr(PhysAddr::new(node.as_ptr() as usize)))
    }
    pub fn free(&mut self, page: PhysPage<Size4K>) {
        let node = Node { next: self.dirty };
        let ptr: *mut Node = page
            .addr()
            .to_virt_offset(*super::HHDM_START.get().unwrap())
            .into_ptr()
            .get();
        unsafe {
            ptr.write(node);
        }
        self.dirty = NonNull::new(ptr);
    }
    /// Cleans a single dirty page, and puts it in the freelist.
    /// Returns whether or not there is another dirty page.
    pub fn clean_dirty(&mut self) -> bool {
        let Some(mut dirty) = self.dirty else {
            return false;
        };

        let ptr = dirty.as_ptr().wrapping_add(1);
        unsafe {
            ptr.write_bytes(0, 4096 - size_of!(Node));
        }

        unsafe { dirty.as_mut().next = self.free };
        self.free = Some(dirty);
        self.dirty.is_some()
    }
}
impl Debug for PhysAlloc {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut node = self.free;
        let mut free_count = 0;
        while let Some(n) = node {
            free_count += 1;
            node = unsafe { n.as_ref().next };
        }
        let mut node = self.dirty;
        let mut dirty_count = 0;
        while let Some(n) = node {
            dirty_count += 1;
            node = unsafe { n.as_ref().next };
        }
        f.debug_struct("PhysAlloc")
            .field(
                "free",
                &format_args!("{free_count} pages ({})", Size(free_count * 4096)),
            )
            .field(
                "dirty",
                &format_args!("{dirty_count} pages ({})", Size(dirty_count * 4096)),
            )
            .finish();
        Ok(())
    }
}
