use core::{mem::MaybeUninit, ptr::NonNull};

use alloc::{boxed::Box, vec::Vec};
use system::{cpus::CpuInfo, sync::Mutex};

pub trait Alloc {
    type Item;
    fn alloc() -> Option<Self::Item>;
    fn free(item: Self::Item);
}

pub struct Heap<T> {
    _marker: core::marker::PhantomData<NonNull<T>>,
}
impl<T> Alloc for Heap<T> {
    type Item = NonNull<T>;
    fn alloc() -> Option<Self::Item> {
        let item = Box::into_raw(Box::new(MaybeUninit::<T>::uninit())) as *mut T;
        NonNull::new(item)
    }
    fn free(item: Self::Item) {
        unsafe { Box::from_raw(item.as_ptr() as *mut MaybeUninit<T>) };
    }
}

pub struct Slab<T: Alloc, const N: usize, const LOCKFREE: usize> {
    inner: Mutex<SlabInner<T, N, LOCKFREE>>,
}
impl<T: Alloc, const N: usize, const LOCKFREE: usize> Slab<T, N, LOCKFREE> {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(SlabInner::new()),
        }
    }

    pub fn alloc_lockfree(&self) -> Option<T::Item> {
        let inner = unsafe { self.inner.get_unchecked_mut() };
        inner.alloc_lockfree()
    }
    pub fn free_lockfree(&self, item: T::Item) -> Result<(), (Full, T::Item)> {
        let inner = unsafe { self.inner.get_unchecked_mut() };
        inner.free_lockfree(item)
    }

    pub async fn alloc(&self) -> Option<T::Item> {
        let inner = unsafe { self.inner.get_unchecked_mut() };
        if let Some(item) = inner.alloc_lockfree() {
            Some(item)
        } else {
            let mut lock = self.inner.lock().await;
            let item = lock.alloc();
            unsafe { self.inner.unlock() };
            item
        }
    }
    pub async fn free(&self, item: T::Item) {
        let inner = unsafe { self.inner.get_unchecked_mut() };
        match inner.free_lockfree(item) {
            Err((Full, item)) => {
                let mut lock = self.inner.lock().await;
                lock.free(item);
            }
            _ => (),
        }
    }
    pub async fn restock(&self) {
        let mut lock = self.inner.lock().await;
        lock.restock();
    }

    pub async fn alloc_or_restock(&self) -> Option<T::Item> {
        match self.alloc().await {
            Some(item) => Some(item),
            None => {
                self.restock().await;
                self.alloc().await
            }
        }
    }
}

struct SlabInner<T: Alloc, const N: usize, const LOCKFREE: usize> {
    items: heapless::Vec<T::Item, N>,
    lock_free: Box<[LockFreeSlab<T::Item, LOCKFREE>]>,
}
impl<T: Alloc, const N: usize, const LOCKFREE: usize> SlabInner<T, N, LOCKFREE> {
    pub fn new() -> Self {
        let vec: Vec<_> = (0..CpuInfo::num_cpus())
            .map(|_| LockFreeSlab::new())
            .collect();
        Self::from_slice(vec.into_boxed_slice()).unwrap()
    }
    pub fn from_slice(slice: Box<[LockFreeSlab<T::Item, LOCKFREE>]>) -> Option<Self> {
        if slice.len() != CpuInfo::num_cpus() {
            return None;
        } else {
            Some(Self {
                items: heapless::Vec::new(),
                lock_free: slice,
            })
        }
    }
    pub fn alloc_lockfree(&mut self) -> Option<T::Item> {
        let cpu = CpuInfo::cpu_id();
        self.lock_free[cpu].alloc()
    }
    pub fn free_lockfree(&mut self, item: T::Item) -> Result<(), (Full, T::Item)> {
        let cpu = CpuInfo::cpu_id();
        self.lock_free[cpu].free(item)
    }
    pub fn alloc(&mut self) -> Option<T::Item> {
        self.items.pop()
    }
    pub fn free(&mut self, item: T::Item) {
        if self.needed() > 0 {
            let _ = self.items.push(item);
        } else {
            let cpu = CpuInfo::cpu_id();
            if self.lock_free[cpu].needed() > 0 {
                let _ = self.lock_free[cpu].free(item);
            } else {
                T::free(item)
            }
        }
    }
    pub fn needed(&self) -> usize {
        N - self.items.len()
    }
    pub fn restock(&mut self) {
        let needed = self.needed();
        for _ in 0..needed {
            if let Some(item) = T::alloc() {
                let _ = self.items.push(item);
            } else {
                break;
            }
        }
        let cpu = CpuInfo::cpu_id();
        let cpu_needed = self.lock_free[cpu].needed();
        let iter = (0..cpu_needed).map(|_| T::alloc()).flatten();
        self.lock_free[cpu].restock(iter);
    }
}

pub struct Full;

struct LockFreeSlab<T, const N: usize> {
    pub items: heapless::Vec<T, N>,
}
impl<T, const N: usize> LockFreeSlab<T, N> {
    pub fn new() -> Self {
        Self {
            items: heapless::Vec::new(),
        }
    }

    pub fn restock(&mut self, items: impl Iterator<Item = T>) {
        self.items.extend(items);
    }

    pub fn alloc(&mut self) -> Option<T> {
        self.items.pop()
    }

    pub fn free(&mut self, item: T) -> Result<(), (Full, T)> {
        self.items.push(item).map_err(|item| (Full, item))
    }

    pub fn needed(&self) -> usize {
        N - self.items.len()
    }
}
