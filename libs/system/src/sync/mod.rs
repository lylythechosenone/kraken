use core::{
    cell::UnsafeCell,
    future::Future,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll, Waker},
};

use pin_list::{Node, NodeData, PinList};
use pin_project::{pin_project, pinned_drop};

type PinListTypes = dyn pin_list::Types<
    Id = pin_list::id::Checked,
    Protected = Waker,
    Removed = (),
    Unprotected = (),
>;

pub struct Mutex<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
    wakers: spin::Mutex<PinList<PinListTypes>>,
}
impl<T> Mutex<T> {
    pub fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
            wakers: spin::Mutex::new(PinList::new(pin_list::id::Checked::new())),
        }
    }

    pub fn lock<'a>(&'a self) -> impl Future<Output = Lock<'a, T>> + 'a
    where
        T: 'a,
    {
        LockFuture {
            mutex: self,
            waker: Node::new(),
        }
    }
    pub unsafe fn get_unchecked(&self) -> &T {
        &*self.data.get()
    }
    pub unsafe fn get_unchecked_mut(&self) -> &mut T {
        &mut *self.data.get()
    }
    pub unsafe fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
        let mut lock = self.wakers.lock();
        if let Ok(waker) = lock.cursor_front_mut().remove_current(()) {
            drop(lock);
            waker.wake();
        }
    }
}

#[pin_project(PinnedDrop)]
pub struct LockFuture<'a, T> {
    mutex: &'a Mutex<T>,
    #[pin]
    waker: Node<PinListTypes>,
}
impl<'a, T> Future for LockFuture<'a, T> {
    type Output = Lock<'a, T>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> core::task::Poll<Self::Output> {
        let mut lock = self.mutex.wakers.lock();
        let mut projected = self.project();
        if let Some(initialized) = projected.waker.as_mut().initialized_mut() {
            if let Err(node) = initialized.take_removed(&lock) {
                *node.protected_mut(&mut lock).unwrap() = cx.waker().clone();
                return Poll::Pending;
            }
        }

        if projected
            .mutex
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Poll::Ready(Lock {
                mutex: projected.mutex,
            })
        } else {
            lock.push_front(projected.waker, cx.waker().clone(), ());
            Poll::Pending
        }
    }
}
#[pinned_drop]
impl<'a, T> PinnedDrop for LockFuture<'a, T> {
    fn drop(self: Pin<&mut Self>) {
        let projected = self.project();
        let node = match projected.waker.initialized_mut() {
            Some(initialized) => initialized,
            None => return,
        };

        let mut lock = projected.mutex.wakers.lock();

        match node.reset(&mut lock) {
            (NodeData::Linked(_waker), ()) => {}
            (NodeData::Removed(()), ()) => {
                if let Ok(waker) = lock.cursor_front_mut().remove_current(()) {
                    drop(lock);
                    waker.wake();
                }
            }
        }
    }
}

pub struct Lock<'a, T> {
    mutex: &'a Mutex<T>,
}
impl<'a, T> Lock<'a, T> {
    pub fn get(&self) -> &'a T {
        unsafe { self.mutex.get_unchecked() }
    }
    pub fn get_mut(&mut self) -> &'a mut T {
        unsafe { self.mutex.get_unchecked_mut() }
    }
}
impl<'a, T> Deref for Lock<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}
impl<'a, T> DerefMut for Lock<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}
impl<'a, T> Drop for Lock<'a, T> {
    fn drop(&mut self) {
        unsafe { self.mutex.unlock() }
    }
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}
