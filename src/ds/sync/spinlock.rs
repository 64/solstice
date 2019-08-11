use crate::cpu::percpu::PerCpu;
use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{spin_loop_hint, AtomicBool, Ordering},
};

pub struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for SpinLock<T> {}
unsafe impl<T: Send> Send for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> SpinLockGuard<T> {
        // Acquire the lock
        unsafe { PerCpu::current().preempt_inc() };
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

    pub fn try_lock(&self) -> Option<SpinLockGuard<T>> {
        unsafe { PerCpu::current().preempt_inc() };

        if !self.locked.compare_and_swap(false, true, Ordering::Acquire) {
            Some(SpinLockGuard {
                locked: &self.locked,
                data: unsafe { &mut *self.data.get() },
            })
        } else {
            unsafe { PerCpu::current().preempt_dec() };
            None
        }
    }
}

impl<T: Default> Default for SpinLock<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for SpinLock<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self.try_lock() {
            Some(temp) => f
                .debug_struct("SpinLock")
                .field("data", &temp.data)
                .finish(),
            None => f
                .debug_struct("SpinLock")
                .field("data", b"<locked>")
                .finish(),
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
        unsafe { PerCpu::current().preempt_dec() };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    test_case!(lock, {
        let m = SpinLock::new(());
        {
            let l = m.try_lock();
            assert!(l.is_some());
            let l2 = m.try_lock();
            assert!(l2.is_none());
        }

        let _l = m.lock();
        let l2 = m.try_lock();
        assert!(l2.is_none());
    });

    test_case!(preempt_count, {
        let pc = || PerCpu::current().preempt_count(core::sync::atomic::Ordering::SeqCst);
        assert_eq!(pc(), 0);

        let m = SpinLock::new(());

        {
            let _l = m.lock();
            assert_eq!(pc(), 1);
            let _l2 = m.try_lock();
            assert_eq!(pc(), 1);
        }
        assert_eq!(pc(), 0);

        {
            let _l = m.try_lock();
            assert_eq!(pc(), 1);
        }
        assert_eq!(pc(), 0);
    });
}
