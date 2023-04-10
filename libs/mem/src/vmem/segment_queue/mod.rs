use core::{marker::PhantomData, mem::MaybeUninit, ptr::NonNull};

use super::{Bt, BtKind, Link};

pub mod allocation_table;
pub mod freelists;

pub struct SegmentQueue {
    pub head: Option<NonNull<Bt>>,
    pub tail: Option<NonNull<Bt>>,
}
impl Default for SegmentQueue {
    fn default() -> Self {
        Self::new()
    }
}
impl SegmentQueue {
    pub const fn new() -> Self {
        Self {
            head: None,
            tail: None,
        }
    }
    pub fn add(&mut self, mut bt: NonNull<Bt>) {
        match unsafe { bt.as_ref() }.kind {
            BtKind::Free => {}
            BtKind::Used => {}
            _ => {
                panic!("Attempted to add a non-free/used segment to the queue");
            }
        }
        let bt_mut = unsafe { bt.as_mut() };
        bt_mut.segment_queue = MaybeUninit::new(Link {
            next: None,
            prev: self.tail,
        });
        if let Some(mut tail) = self.tail {
            unsafe { tail.as_mut().segment_queue.assume_init_mut() }.next = Some(bt);
        }
        self.tail = Some(bt);
        if self.head.is_none() {
            self.head = Some(bt);
        }
    }
    pub fn remove(&mut self, mut bt: NonNull<Bt>) {
        match unsafe { bt.as_ref() }.kind {
            BtKind::Free => {}
            BtKind::Used => {}
            _ => {
                panic!("Attempted to remove a non-free/used segment from the queue");
            }
        }
        let bt_mut = unsafe { bt.as_mut() };
        if let Some(mut prev) = unsafe { bt_mut.segment_queue.assume_init() }.prev {
            unsafe { prev.as_mut().segment_queue.assume_init_mut() }.next =
                unsafe { bt_mut.segment_queue.assume_init() }.next;
        }
        if let Some(mut next) = unsafe { bt_mut.segment_queue.assume_init() }.next {
            unsafe { next.as_mut().segment_queue.assume_init_mut() }.prev =
                unsafe { bt_mut.segment_queue.assume_init() }.prev;
        }
        if self.head == Some(bt) {
            self.head = unsafe { bt_mut.segment_queue.assume_init() }.next;
        }
        if self.tail == Some(bt) {
            self.tail = unsafe { bt_mut.segment_queue.assume_init() }.prev;
        }
        bt_mut.segment_queue = MaybeUninit::uninit();
    }
    pub fn push(&mut self, bt: NonNull<Bt>) {
        self.add(bt);
    }
    pub fn pop(&mut self) -> Option<NonNull<Bt>> {
        let bt = self.head;
        if let Some(bt) = bt {
            self.remove(bt);
        }
        bt
    }
    pub fn iter(&self) -> SegmentQueueIter {
        SegmentQueueIter::new(self)
    }
}

pub struct SegmentQueueIter<'a>(Option<NonNull<Bt>>, PhantomData<&'a SegmentQueue>);
impl<'a> SegmentQueueIter<'a> {
    pub fn new(queue: &'a SegmentQueue) -> Self {
        Self(queue.head, PhantomData)
    }
}
impl<'a> Iterator for SegmentQueueIter<'a> {
    type Item = NonNull<Bt>;

    fn next(&mut self) -> Option<Self::Item> {
        let bt = self.0;
        if let Some(bt) = bt {
            self.0 = unsafe { bt.as_ref().segment_queue.assume_init() }.next;
        }
        bt
    }
}
impl<'a> IntoIterator for &'a SegmentQueue {
    type Item = NonNull<Bt>;
    type IntoIter = SegmentQueueIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter::new(self)
    }
}
