use alloc::{vec, vec::Vec};

pub trait Alloc {
    type Item: Clone;
    async fn alloc() -> Option<Self::Item>;
    async fn free(item: Self::Item);
}

pub struct Slab<A: Alloc, const N: usize> {
    slabs: Vec<heapless::Vec<A::Item, N>>,
}

unsafe impl<A: Alloc, const N: usize> Send for Slab<A, N> {}
unsafe impl<A: Alloc, const N: usize> Sync for Slab<A, N> {}

impl<A: Alloc, const N: usize> Slab<A, N> {
    pub fn new() -> Self {
        Self {
            slabs: vec![heapless::Vec::new(); system::cpus::CpuInfo::num_cpus()],
        }
    }

    pub fn alloc(&mut self) -> Option<A::Item> {
        let cpu_id = system::cpus::CpuInfo::cpu_id();
        let slab = &mut self.slabs[cpu_id];
        if let Some(item) = slab.pop() {
            Some(item)
        } else {
            None
        }
    }

    pub async fn free(&mut self, item: A::Item) {
        let cpu_id = system::cpus::CpuInfo::cpu_id();
        let slab = &mut self.slabs[cpu_id];
        if let Err(item) = slab.push(item) {
            A::free(item).await;
        }
    }
    pub fn free_nolock(&mut self, item: A::Item) -> Result<(), A::Item> {
        let cpu_id = system::cpus::CpuInfo::cpu_id();
        let slab = &mut self.slabs[cpu_id];
        slab.push(item)
    }

    pub async fn restock(&mut self) -> bool {
        let cpu_id = system::cpus::CpuInfo::cpu_id();
        let slab = &mut self.slabs[cpu_id];
        for _ in 0..N - slab.len() {
            if let Some(item) = A::alloc().await {
                let _ = slab.push(item);
            } else {
                return false;
            }
        }
        true
    }

    pub async fn alloc_restocking(&mut self) -> Option<A::Item> {
        if let Some(item) = self.alloc() {
            Some(item)
        } else {
            self.restock().await;
            self.alloc()
        }
    }
}
