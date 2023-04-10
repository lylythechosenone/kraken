use core::ptr::NonNull;

use super::Bt;

pub struct SegmentList {
    pub head: Option<NonNull<Bt>>,
    pub tail: Option<NonNull<Bt>>,
}
impl Default for SegmentList {
    fn default() -> Self {
        Self::new()
    }
}
impl SegmentList {
    pub const fn new() -> Self {
        Self {
            head: None,
            tail: None,
        }
    }
    pub fn add(&mut self, mut bt: NonNull<Bt>) {
        let bt_mut = unsafe { bt.as_mut() };
        bt_mut.segment_list.next = None;
        bt_mut.segment_list.prev = self.tail;
        if let Some(mut tail) = self.tail {
            unsafe { tail.as_mut() }.segment_list.next = Some(bt);
        }
        self.tail = Some(bt);
        if self.head.is_none() {
            self.head = Some(bt);
        }
    }
    pub fn remove(&mut self, mut bt: NonNull<Bt>) {
        let bt_mut = unsafe { bt.as_mut() };
        if let Some(mut prev) = bt_mut.segment_list.prev {
            unsafe { prev.as_mut() }.segment_list.next = bt_mut.segment_list.next;
        }
        if let Some(mut next) = bt_mut.segment_list.next {
            unsafe { next.as_mut() }.segment_list.prev = bt_mut.segment_list.prev;
        }
        if self.head == Some(bt) {
            self.head = bt_mut.segment_list.next;
        }
        if self.tail == Some(bt) {
            self.tail = bt_mut.segment_list.prev;
        }
        bt_mut.segment_list.next = None;
        bt_mut.segment_list.prev = None;
    }
    pub fn insert_before(&mut self, mut new: NonNull<Bt>, mut old: NonNull<Bt>) {
        let new_mut = unsafe { new.as_mut() };
        let old_mut = unsafe { old.as_mut() };
        new_mut.segment_list.next = Some(old);
        new_mut.segment_list.prev = old_mut.segment_list.prev;
        if let Some(mut prev) = old_mut.segment_list.prev {
            unsafe { prev.as_mut() }.segment_list.next = Some(new);
        }
        old_mut.segment_list.prev = Some(new);
        if self.head == Some(old) {
            self.head = Some(new);
        }
    }
    pub fn first(&self) -> Option<NonNull<Bt>> {
        self.head
    }
    pub fn next(&self, bt: NonNull<Bt>) -> Option<NonNull<Bt>> {
        unsafe { bt.as_ref() }.segment_list.next
    }
    pub fn iter(&self) -> SegmentListIter {
        SegmentListIter::new(self)
    }
    pub fn iter_from(&self, bt: NonNull<Bt>) -> SegmentListIter {
        SegmentListIter::from(bt)
    }
}

pub struct SegmentListIter(Option<NonNull<Bt>>);
impl SegmentListIter {
    pub fn new(list: &SegmentList) -> Self {
        Self(list.head)
    }
    pub fn from(bt: NonNull<Bt>) -> Self {
        Self(Some(bt))
    }
}
impl Iterator for SegmentListIter {
    type Item = NonNull<Bt>;

    fn next(&mut self) -> Option<Self::Item> {
        let bt = self.0;
        if let Some(bt) = bt {
            self.0 = unsafe { bt.as_ref() }.segment_list.next;
        }
        bt
    }
}
impl DoubleEndedIterator for SegmentListIter {
    fn next_back(&mut self) -> Option<Self::Item> {
        let bt = self.0;
        if let Some(bt) = bt {
            self.0 = unsafe { bt.as_ref() }.segment_list.prev;
        }
        bt
    }
}
impl IntoIterator for &SegmentList {
    type Item = NonNull<Bt>;
    type IntoIter = SegmentListIter;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter::new(self)
    }
}
