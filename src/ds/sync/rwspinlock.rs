use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{spin_loop_hint, AtomicUsize, Ordering},
};

pub struct RwSpinLock<T> {
    lock: AtomicUsize,
    data: UnsafeCell<T>,
}

pub struct RwLockWriteGuard<'a, T: 'a> {
    lock: &'a AtomicUsize,
    data: &'a mut T,
}

pub struct RwLockReadGuard<'a, T: 'a> {
    lock: &'a AtomicUsize,
    data: &'a T,
}

const U_MIN: usize = core::isize::MIN as usize;

unsafe impl<T: Send> Send for RwSpinLock<T> {}
unsafe impl<T: Send + Sync> Sync for RwSpinLock<T> {}

impl<T> RwSpinLock<T> {
    pub fn new(d: T) -> RwSpinLock<T> {
        RwSpinLock {
            lock: AtomicUsize::new(0),
            data: UnsafeCell::new(d),
        }
    }
    pub fn get_data(self) -> T {
        let RwSpinLock { data, .. } = self;
        data.into_inner()
    }

    pub fn read(&self) -> RwLockReadGuard<T> {
        while {
            // The old_value hasn't got a "write bit" set yet.
            let mut old_value;
            while {
                // Set old_value's bit.
                old_value = self.lock.load(Ordering::Relaxed);
                // Checking if the most significant bit of "U_MIN" and "old_value" are 0,
                // if so, ending the loop.
                old_value & U_MIN != 0
            } {
                spin_loop_hint();
            }
            // Unset the bit thanks to U_MIN which is the MSB.
            old_value &= !U_MIN;
            let new_value = old_value + 1;
            self.lock
                .compare_and_swap(old_value, new_value, Ordering::SeqCst)
                != old_value
        } {
            spin_loop_hint();
        }
        RwLockReadGuard {
            lock: &self.lock,
            data: unsafe { &*self.data.get() },
        }
    }
    pub fn try_read(&self) -> Option<RwLockReadGuard<T>> {
        let old_value = (!U_MIN) & self.lock.load(Ordering::Relaxed);
        let new_value = old_value + 1;
        if self
            .lock
            .compare_and_swap(old_value, new_value, Ordering::SeqCst)
            == old_value
        {
            Some(RwLockReadGuard {
                lock: &self.lock,
                data: unsafe { &*self.data.get() },
            })
        } else {
            None
        }
    }

    fn force_read_decrement(&self) {
        self.lock.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn force_write_unlock(&self) {
        self.lock.store(0, Ordering::Relaxed);
    }

    pub fn write(&self) -> RwLockWriteGuard<T> {
        loop {
            let old_value = (!U_MIN) & self.lock.load(Ordering::Relaxed);
            let new_value = U_MIN | old_value;
            if self
                .lock
                .compare_and_swap(old_value, new_value, Ordering::SeqCst)
                == old_value
            {
                while self
                    .lock
                    .compare_and_swap(old_value, new_value, Ordering::Relaxed)
                    != U_MIN
                {
                    spin_loop_hint();
                }
                break;
            }
        }
        RwLockWriteGuard {
            lock: &self.lock,
            data: unsafe { &mut *self.data.get() },
        }
    }
    pub fn try_write(&self) -> Option<RwLockWriteGuard<T>> {
        if self.lock.compare_and_swap(0, U_MIN, Ordering::SeqCst) == 0 {
            Some(RwLockWriteGuard {
                lock: &self.lock,
                data: unsafe { &mut *self.data.get() },
            })
        } else {
            None
        }
    }
}

impl<T: Default> Default for RwSpinLock<T> {
    fn default() -> RwSpinLock<T> {
        RwSpinLock::new(Default::default())
    }
}

impl<'a, T> Deref for RwLockReadGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.data
    }
}

impl<'a, T> Deref for RwLockWriteGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.data
    }
}

impl<'a, T> DerefMut for RwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<'a, T> Drop for RwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.fetch_sub(1, Ordering::SeqCst);
    }
}

impl<'a, T> Drop for RwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.store(0, Ordering::Relaxed);
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for RwSpinLock<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self.try_read() {
            Some(temp) => f
                .debug_struct("RwSpinLock")
                .field("data", &temp.data)
                .finish(),
            None => f
                .debug_struct("RwSpinLock")
                .field("data", b"failed reading")
                .finish(),
        }
    }
}

mod tests {
	use core::prelude::v1::*;
	use core::sync::atomic::{AtomicUsize, Ordering};
	use super::*;

	#[derive(Eq, PartialEq, Debug)]
	struct NonCopy(i32);
	#[test]
	fn smoke() {
		let l = RwLock::new(());
		drop(l.read());
		drop(l.write());
		drop((l.read(), l.read()));
		drop(l.write());
	}
	#[test]
	fn test_rwlock_unsized() {
		let rw: &RwLock<[i32]> = &RwLock::new([1, 2, 3]);
		{
			let b = &mut *rw.write();
			b[0] = 4;
			b[2] = 5;
		}
		let comp: &[i32] = &[4, 2, 5];
		assert_eq!(&*rw.read(), comp);
	}
	#[test]
	fn test_rwlock_try_write() {
		use core::mem::drop;
		let lock = RwLock::new(0isize);
		let read_guard = lock.read();
		let write_result = lock.try_write();
		match write_result {
			None => (),
			Some(_) => assert!(false, "Error"),
		}
		drop(read_guard);
	}
}
