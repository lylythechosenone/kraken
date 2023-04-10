use core::ptr::NonNull;

use crate::vmem::Bt;

use super::SegmentQueue;

pub struct Freelists {
    lists: [SegmentQueue; Self::LISTS],
}
impl Freelists {
    #[cfg(target_pointer_width = "32")]
    pub const LISTS: usize = 32;
    #[cfg(target_pointer_width = "64")]
    pub const LISTS: usize = 64;

    pub const fn new() -> Self {
        const TEMP: SegmentQueue = SegmentQueue::new();
        Self {
            lists: [TEMP; Self::LISTS],
        }
    }

    const fn get_list(size: usize) -> usize {
        let po2 = size.next_power_of_two();
        let list = po2.trailing_zeros() as usize;
        list
    }

    pub fn best_fit(&self, size: usize, quantum: usize) -> Option<NonNull<Bt>> {
        let size = (size + (quantum - 1)) / quantum;
        let mut list = Self::get_list(size);
        if !size.is_power_of_two() {
            list -= 1;
        }
        for list in &self.lists[list..] {
            if let Some(min) = list
                .iter()
                .filter(|bt| unsafe { bt.as_ref() }.len >= size)
                .min_by_key(|&bt| unsafe { bt.as_ref() }.len)
            {
                return Some(min);
            }
        }
        None
    }

    pub fn instant_fit(&self, size: usize, quantum: usize) -> Option<NonNull<Bt>> {
        let size = (size + (quantum - 1)) / quantum;
        let list = Self::get_list(size);
        for list in &self.lists[list..] {
            if let Some(fit) = list.iter().next() {
                return Some(fit);
            }
        }
        None
    }

    pub fn insert(&mut self, bt: NonNull<Bt>, quantum: usize) {
        let size = (unsafe { bt.as_ref() }.len + (quantum - 1)) / quantum;
        let list = Self::get_list(size);
        self.lists[list].add(bt);
    }
    pub fn remove(&mut self, bt: NonNull<Bt>, quantum: usize) {
        let size = (unsafe { bt.as_ref() }.len + (quantum - 1)) / quantum;
        let list = Self::get_list(size);
        self.lists[list].remove(bt);
    }
}
