use core::cell::UnsafeCell;

use alloc::{vec, vec::Vec};
use system::sync::{Lock, Mutex};

pub trait Alloc {
    type Item: Clone;
    async fn alloc(&mut self) -> Option<Self::Item>;
    async fn free(&mut self, item: Self::Item);
}

pub struct Slab<A: Alloc, const N: usize> {
    slabs: UnsafeCell<Vec<heapless::Vec<A::Item, N>>>,
    alloc: Mutex<A>,
}

unsafe impl<A: Alloc, const N: usize> Send for Slab<A, N> {}
unsafe impl<A: Alloc, const N: usize> Sync for Slab<A, N> {}

impl<A: Alloc, const N: usize> Slab<A, N> {
    pub fn new(alloc: A) -> Self {
        Self {
            slabs: UnsafeCell::new(vec![
                heapless::Vec::new();
                system::cpus::CpuInfo::num_cpus()
            ]),
            alloc: Mutex::new(alloc),
        }
    }

    pub fn empty(&self) -> bool {
        let cpu_id = system::cpus::CpuInfo::cpu_id();
        let slab = &self.get_slabs()[cpu_id];
        slab.is_empty()
    }

    pub fn alloc(&self) -> Option<A::Item> {
        let cpu_id = system::cpus::CpuInfo::cpu_id();
        let slab = &mut self.get_slabs_mut()[cpu_id];
        if let Some(item) = slab.pop() {
            Some(item)
        } else {
            None
        }
    }

    pub async fn free(&self, item: A::Item) {
        let cpu_id = system::cpus::CpuInfo::cpu_id();
        let slab = &mut self.get_slabs_mut()[cpu_id];
        if let Err(item) = slab.push(item) {
            self.alloc.lock().await.free(item).await;
        }
    }
    pub fn free_nolock(&self, item: A::Item) -> Result<(), A::Item> {
        let cpu_id = system::cpus::CpuInfo::cpu_id();
        let slab = &mut self.get_slabs_mut()[cpu_id];
        slab.push(item)
    }

    pub async fn restock(&self) -> bool {
        let cpu_id = system::cpus::CpuInfo::cpu_id();
        let slab = &mut self.get_slabs_mut()[cpu_id];
        for _ in 0..N - slab.len() {
            if let Some(item) = self.alloc.lock().await.alloc().await {
                let _ = slab.push(item);
            } else {
                return false;
            }
        }
        true
    }

    pub async fn alloc_restocking(&self) -> Option<A::Item> {
        if let Some(item) = self.alloc() {
            Some(item)
        } else {
            self.restock().await;
            self.alloc()
        }
    }

    pub async fn alloc_shortcircuiting(&self) -> Option<A::Item> {
        if let Some(item) = self.alloc() {
            Some(item)
        } else {
            self.alloc.lock().await.alloc().await
        }
    }

    pub async fn lock_alloc(&self) -> Lock<'_, A> {
        self.alloc.lock().await
    }

    fn get_slabs(&self) -> &Vec<heapless::Vec<A::Item, N>> {
        unsafe { &*self.slabs.get() }
    }
    fn get_slabs_mut(&self) -> &mut Vec<heapless::Vec<A::Item, N>> {
        unsafe { &mut *self.slabs.get() }
    }
}
