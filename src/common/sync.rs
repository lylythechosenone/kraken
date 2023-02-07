use core::{cell::UnsafeCell, sync::atomic::AtomicBool, sync::atomic::Ordering};

static IS_SINGLE_CORE: AtomicBool = AtomicBool::new(true);

/// Invalidates all `SingleCoreLock`s.
pub fn start_smp() {
    IS_SINGLE_CORE.store(false, Ordering::Release);
}

/// Makes all `SingleCoreLock`s valid.
///
/// # Safety
/// Don't lie to me.
pub unsafe fn stop_smp() {
    IS_SINGLE_CORE.store(true, Ordering::Release);
}

pub struct SingleCoreLock<T> {
    value: UnsafeCell<T>,
}
unsafe impl<T> Sync for SingleCoreLock<T> {}
impl<T> SingleCoreLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    pub fn get(&self) -> &T {
        unsafe { &*self.value.get() }
    }

    pub fn get_mut(&mut self) -> &mut T {
        if IS_SINGLE_CORE.load(Ordering::Acquire) {
            unsafe { &mut *self.get_unchecked_mut() }
        } else {
            panic!("SingleCoreLock used on SMP");
        }
    }

    pub fn set(&self, value: T) {
        if IS_SINGLE_CORE.load(Ordering::Acquire) {
            unsafe { self.set_unchecked(value) }
        } else {
            panic!("SingleCoreLock used on SMP");
        }
    }

    /// # Safety
    /// The system must be single core mode.
    pub unsafe fn get_unchecked_mut(&mut self) -> &mut T {
        &mut *self.value.get()
    }

    /// # Safety
    /// The system must be single core mode.
    pub unsafe fn set_unchecked(&self, value: T) {
        *self.value.get() = value;
    }
}
