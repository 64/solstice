use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{spin_loop_hint, AtomicBool, Ordering},
};
use core::fmt::Debug;
use core::borrow::Borrow;

pub struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for SpinLock<T> {}
unsafe impl<T: Send> Send for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> SpinLockGuard<T> {
        // Acquire the lock
        while self.locked.compare_and_swap(false, true, Ordering::Acquire) {
            while self.locked.load(Ordering::Relaxed) {
                spin_loop_hint();
            }
        }

        SpinLockGuard {
            locked: &self.locked,
            data: unsafe { &mut *self.data.get() },
        }
    }
}

impl<T: Default> Default for SpinLock<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> SpinLock<T> {
    pub fn try_lock(&self) -> Option<SpinLockGuard<T>> {
        if self.locked.compare_and_swap(false, true, Ordering::Acquire) == false {
            Some(
                SpinLockGuard {
                    locked: &self.locked,
                    data: unsafe { &mut *self.data.get() }
                })
        } else {
            None
        }
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for SpinLock<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self.try_lock() {
            Some(temp) =>
                write!(f, "[Debug Mutex: Data: {:?}\n", temp.data),
            None =>
                write!(f, "[Debug: Mutex is locked]\n")
        }
    }
}

pub struct SpinLockGuard<'a, T> {
    locked: &'a AtomicBool,
    data: &'a mut T,
}

impl<T> Deref for SpinLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.data
    }
}

impl<T> DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<T> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        self.locked.store(false, Ordering::Release);
    }
}