use core::ptr::NonNull;

use crate::vmem::Bt;

use super::SegmentQueue;

pub struct AllocationTable {
    pub buckets: [SegmentQueue; Self::BUCKETS],
}
impl AllocationTable {
    pub const BUCKETS: usize = 64;

    pub const fn new() -> Self {
        const TEMP: SegmentQueue = SegmentQueue::new();
        Self {
            buckets: [TEMP; Self::BUCKETS],
        }
    }

    const fn get_bucket(n: usize) -> usize {
        Self::murmur(n) % Self::BUCKETS
    }

    pub fn insert(&mut self, bt: NonNull<Bt>) {
        let bucket = Self::get_bucket(unsafe { bt.as_ref() }.base);
        self.buckets[bucket].add(bt);
    }
    pub fn remove(&mut self, bt: NonNull<Bt>) {
        let bucket = Self::get_bucket(unsafe { bt.as_ref() }.base);
        self.buckets[bucket].remove(bt);
    }

    pub fn get(&self, base: usize) -> Option<NonNull<Bt>> {
        let bucket = Self::get_bucket(base);
        self.buckets[bucket]
            .iter()
            .find(|&bt| unsafe { bt.as_ref() }.base == base)
    }

    #[cfg(target_pointer_width = "64")]
    const fn murmur(mut key: usize) -> usize {
        key ^= key >> 33;
        key = key.wrapping_mul(0xff51afd7ed558ccd);
        key ^= key >> 33;
        key = key.wrapping_mul(0xc4ceb9fe1a85ec53);
        key ^= key >> 33;
        key
    }
    #[cfg(target_pointer_width = "32")]
    const fn murmur(mut key: usize) -> usize {
        key ^= key >> 16;
        key = key.wrapping_mul(0x85ebca6b);
        key ^= key >> 13;
        key = key.wrapping_mul(0xc2b2ae35);
        key ^= key >> 16;
        key
    }
}
