use core::{mem::MaybeUninit, ptr::NonNull};

use system::sync::Mutex;

use crate::slab::{Heap, Slab};

use self::{
    segment_list::SegmentList,
    segment_queue::{allocation_table::AllocationTable, freelists::Freelists},
};

pub mod segment_list;
pub mod segment_queue;

#[derive(Copy, Clone)]
pub struct Link {
    pub next: Option<NonNull<Bt>>,
    pub prev: Option<NonNull<Bt>>,
}

#[derive(PartialEq, Eq)]
pub enum BtKind {
    Span,
    ImportedSpan,
    Free,
    Used,
}

pub struct Bt {
    pub kind: BtKind,

    pub base: usize,
    pub len: usize,

    pub segment_list: Link,
    pub segment_queue: MaybeUninit<Link>,
}

pub enum AllocPolicy {
    InstantFit,
    BestFit,
    NextFit,
}

pub struct Vmem<'src> {
    inner: Mutex<VmemInner<'src>>,
}
impl<'src> Vmem<'src> {
    pub fn new(slab: Slab<Heap<Bt>, 16, 4>, quantum: usize) -> Self {
        Self {
            inner: Mutex::new(VmemInner::new(slab, quantum)),
        }
    }

    pub async fn add_span(&self, base: usize, len: usize) -> &Vmem<'src> {
        let mut inner = self.inner.lock().await;
        inner.add_span(base, len).await;
        self
    }
    pub async fn borrow_span(&self, base: usize, len: usize) -> &Vmem<'src> {
        let mut inner = self.inner.lock().await;
        inner.borrow_span(base, len).await;
        self
    }

    pub async fn set_parent(&self, parent: &'src Vmem<'src>) -> &Vmem<'src> {
        let mut inner = self.inner.lock().await;
        inner.set_parent(parent);
        self
    }

    pub async fn alloc(&self, len: usize, policy: AllocPolicy) -> Option<usize> {
        let mut inner = self.inner.lock().await;
        inner.alloc(policy, len).await
    }

    pub async fn free(&self, base: usize) {
        let mut inner = self.inner.lock().await;
        inner.free(base).await;
    }
}

struct VmemInner<'src> {
    segment_list: SegmentList,
    allocation_table: AllocationTable,
    freelists: Freelists,
    slab: Slab<Heap<Bt>, 16, 4>,
    quantum: usize,
    parent: Option<&'src Vmem<'src>>,
    last: Option<NonNull<Bt>>,
}
impl<'src> VmemInner<'src> {
    pub fn new(slab: Slab<Heap<Bt>, 16, 4>, quantum: usize) -> Self {
        Self {
            segment_list: SegmentList::new(),
            allocation_table: AllocationTable::new(),
            freelists: Freelists::new(),
            slab,
            quantum,
            parent: None,
            last: None,
        }
    }
    pub async fn add_span(&mut self, base: usize, len: usize) {
        let span = self.slab.alloc_or_restock().await.unwrap();
        unsafe {
            *span.as_ptr() = Bt {
                kind: BtKind::Span,
                base,
                len,
                segment_list: Link {
                    next: None,
                    prev: None,
                },
                segment_queue: MaybeUninit::uninit(),
            };
        }
        let initial_segment = self.slab.alloc_or_restock().await.unwrap();
        unsafe {
            *initial_segment.as_ptr() = Bt {
                kind: BtKind::Free,
                base,
                len,
                segment_list: Link {
                    next: None,
                    prev: None,
                },
                segment_queue: MaybeUninit::new(Link {
                    next: None,
                    prev: None,
                }),
            }
        }
        self.segment_list.add(span);
        self.segment_list.add(initial_segment);
        self.freelists.insert(initial_segment, self.quantum);
    }

    pub async fn borrow_span(&mut self, base: usize, len: usize) {
        if self.parent.is_none() {
            panic!("Attempting to borrow span from vmem with no parent");
        }
        let span = self.slab.alloc_or_restock().await.unwrap();
        unsafe {
            *span.as_ptr() = Bt {
                kind: BtKind::ImportedSpan,
                base,
                len,
                segment_list: Link {
                    next: None,
                    prev: None,
                },
                segment_queue: MaybeUninit::uninit(),
            };
        }
        self.segment_list.add(span);
    }

    pub fn set_parent(&mut self, parent: &'src Vmem<'src>) {
        if self.parent.is_some() {
            panic!("Attempting to change existing parent of vmem");
        }
        self.parent = Some(parent);
    }

    pub async fn alloc(&mut self, policy: AllocPolicy, size: usize) -> Option<usize> {
        let mut tag = match policy {
            AllocPolicy::InstantFit => self.freelists.instant_fit(size, self.quantum)?,
            AllocPolicy::BestFit => self.freelists.best_fit(size, self.quantum)?,
            AllocPolicy::NextFit => {
                let next = match self.last {
                    Some(last) => self
                        .segment_list
                        .next(last)
                        .or_else(|| self.freelists.instant_fit(size, self.quantum))?,
                    None => self.segment_list.first()?,
                };
                let mut final_tag = None;
                for tag in self.segment_list.iter_from(next) {
                    if unsafe { tag.as_ref() }.len >= size {
                        final_tag = Some(tag);
                        break;
                    }
                }
                final_tag?
            }
        };
        let tag_mut = unsafe { tag.as_mut() };
        let base = tag_mut.base;
        tag_mut.base += size;
        tag_mut.len -= size;
        let new_tag = self.slab.alloc_or_restock().await.unwrap();
        unsafe {
            *new_tag.as_ptr() = Bt {
                kind: BtKind::Used,
                base,
                len: size,
                segment_list: Link {
                    next: None,
                    prev: None,
                },
                segment_queue: MaybeUninit::new(Link {
                    next: None,
                    prev: None,
                }),
            };
        }
        self.segment_list.insert_before(new_tag, tag);
        self.allocation_table.insert(new_tag);
        self.last = Some(new_tag);
        Some(base)
    }

    pub async fn free(&mut self, base: usize) {
        let mut tag = self.allocation_table.get(base).unwrap();
        let tag_mut = unsafe { tag.as_mut() };
        tag_mut.kind = BtKind::Free;
        self.allocation_table.remove(tag);
        for new_tag in self.segment_list.iter_from(tag).skip(1) {
            let new_tag_ref = unsafe { new_tag.as_ref() };
            if new_tag_ref.kind != BtKind::Free {
                break;
            }
            tag_mut.len += new_tag_ref.len;
            self.segment_list.remove(tag);
            self.freelists.remove(tag, self.quantum);
            self.slab.free(tag).await;
        }
        for new_tag in self.segment_list.iter_from(tag).rev().skip(1) {
            let new_tag_ref = unsafe { new_tag.as_ref() };
            if new_tag_ref.kind != BtKind::Free {
                break;
            }
            tag_mut.base -= new_tag_ref.len;
            tag_mut.len += new_tag_ref.len;
            self.segment_list.remove(tag);
            self.freelists.remove(tag, self.quantum);
            self.slab.free(tag).await;
        }
        self.freelists.insert(tag, self.quantum);
    }
}
