use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{spin_loop_hint, AtomicBool, Ordering, AtomicUsize},
};

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

    pub fn try_lock(&self) -> Option<SpinLockGuard<T>> {
        if !self.locked.compare_and_swap(false, true, Ordering::Acquire) {
            Some(SpinLockGuard {
                locked: &self.locked,
                data: unsafe { &mut *self.data.get() },
            })
        } else {
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
    }
}

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
			let mut old_value;
			while {
				old_value = self.lock.load(Ordering::Relaxed);
				old_value & U_MIN != 0
			} {
				spin_loop_hint();
			}
			old_value &= !U_MIN;
			let new_value = old_value + 1;
			self.lock.compare_and_swap(old_value, new_value, Ordering::SeqCst) != old_value
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
		if self.lock.compare_and_swap(old_value, new_value, Ordering::SeqCst) == old_value {
			Some(RwLockReadGuard {
				lock: &self.lock,
				data: unsafe { &*self.data.get() },
			})
		} else {
			None
		}
	}

	pub fn force_read_decrement(&self) {
		self.lock.fetch_sub(1, Ordering::SeqCst);
	}

	pub fn force_write_unlock(&self) {
		self.lock.store(0, Ordering::Relaxed);
	}

	pub fn write(&self) -> RwLockWriteGuard<T> {
		loop {
			let old_value = (!U_MIN) & self.lock.load(Ordering::Relaxed);
			let new_value = U_MIN | old_value;
			if self.lock.compare_and_swap(old_value, new_value, Ordering::SeqCst) == old_value {
				while self.lock.compare_and_swap(old_value, new_value, Ordering::Relaxed) != U_MIN {
					spin_loop_hint();
				}
				break
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
	fn deref(&self) -> &T { self.data }
}

impl<'a, T> Deref for RwLockWriteGuard<'a, T> {
	type Target = T;
	fn deref(&self) -> &T { self.data }
}

impl<'a, T> DerefMut for RwLockWriteGuard<'a, T> {
	fn deref_mut(&mut self) -> &mut T { self.data }
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


// TODO: Add proper tests
test_case!(spin_lock, {
    assert_eq!(1, 1);
});
