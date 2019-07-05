#![allow(dead_code)]

use core::{
    cell::UnsafeCell,
    default::Default,
    fmt,
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut},
    ptr::NonNull,
    sync::atomic::{spin_loop_hint as cpu_relax, AtomicUsize, Ordering},
};

pub struct RwSpinLock<T: ?Sized> {
    lock: AtomicUsize,
    data: UnsafeCell<T>,
}

const READER: usize = 1 << 2;
const UPGRADED: usize = 1 << 1;
const WRITER: usize = 1;

#[derive(Debug)]
pub struct RwSpinLockReadGuard<'a, T: 'a + ?Sized> {
    lock: &'a AtomicUsize,
    data: NonNull<T>,
}

#[derive(Debug)]
pub struct RwSpinLockWriteGuard<'a, T: 'a + ?Sized> {
    lock: &'a AtomicUsize,
    data: NonNull<T>,
    #[doc(hidden)]
    _invariant: PhantomData<&'a mut T>,
}

#[derive(Debug)]
pub struct RwSpinLockUpgradeableGuard<'a, T: 'a + ?Sized> {
    lock: &'a AtomicUsize,
    data: NonNull<T>,
    #[doc(hidden)]
    _invariant: PhantomData<&'a mut T>,
}

// Same unsafe impls as `std::sync::RwSpinLock`
unsafe impl<T: ?Sized + Send> Send for RwSpinLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for RwSpinLock<T> {}

impl<T> RwSpinLock<T> {
    #[inline]
    pub const fn new(user_data: T) -> RwSpinLock<T> {
        RwSpinLock {
            lock: AtomicUsize::new(0),
            data: UnsafeCell::new(user_data),
        }
    }

    #[inline]
    pub fn into_inner(self) -> T {
        // We know statically that there are no outstanding references to
        // `self` so there's no need to lock.
        let RwSpinLock { data, .. } = self;
        data.into_inner()
    }
}

impl<T: ?Sized> RwSpinLock<T> {
    #[inline]
    pub fn read(&self) -> RwSpinLockReadGuard<T> {
        loop {
            match self.try_read() {
                Some(guard) => return guard,
                None => cpu_relax(),
            }
        }
    }

    #[inline]
    pub fn try_read(&self) -> Option<RwSpinLockReadGuard<T>> {
        let value = self.lock.fetch_add(READER, Ordering::Acquire);

        // We check the UPGRADED bit here so that new readers are prevented when an
        // UPGRADED lock is held. This helps reduce writer starvation.
        if value & (WRITER | UPGRADED) != 0 {
            // Lock is taken, undo.
            self.lock.fetch_sub(READER, Ordering::Release);
            None
        } else {
            Some(RwSpinLockReadGuard {
                lock: &self.lock,
                data: unsafe { NonNull::new_unchecked(self.data.get()) },
            })
        }
    }

    #[inline]
    pub unsafe fn force_read_decrement(&self) {
        debug_assert!(self.lock.load(Ordering::Relaxed) & !WRITER > 0);
        self.lock.fetch_sub(READER, Ordering::Release);
    }

    #[inline]
    pub unsafe fn force_write_unlock(&self) {
        debug_assert_eq!(self.lock.load(Ordering::Relaxed) & !(WRITER | UPGRADED), 0);
        self.lock.fetch_and(!(WRITER | UPGRADED), Ordering::Release);
    }

    #[inline(always)]
    fn try_write_internal(&self, strong: bool) -> Option<RwSpinLockWriteGuard<T>> {
        if compare_exchange(
            &self.lock,
            0,
            WRITER,
            Ordering::Acquire,
            Ordering::Relaxed,
            strong,
        )
        .is_ok()
        {
            Some(RwSpinLockWriteGuard {
                lock: &self.lock,
                data: unsafe { NonNull::new_unchecked(self.data.get()) },
                _invariant: PhantomData,
            })
        } else {
            None
        }
    }

    #[inline]
    pub fn write(&self) -> RwSpinLockWriteGuard<T> {
        loop {
            match self.try_write_internal(false) {
                Some(guard) => return guard,
                None => cpu_relax(),
            }
        }
    }

    #[inline]
    pub fn try_write(&self) -> Option<RwSpinLockWriteGuard<T>> {
        self.try_write_internal(true)
    }

    #[inline]
    pub fn upgradeable_read(&self) -> RwSpinLockUpgradeableGuard<T> {
        loop {
            match self.try_upgradeable_read() {
                Some(guard) => return guard,
                None => cpu_relax(),
            }
        }
    }

    #[inline]
    pub fn try_upgradeable_read(&self) -> Option<RwSpinLockUpgradeableGuard<T>> {
        if self.lock.fetch_or(UPGRADED, Ordering::Acquire) & (WRITER | UPGRADED) == 0 {
            Some(RwSpinLockUpgradeableGuard {
                lock: &self.lock,
                data: unsafe { NonNull::new_unchecked(self.data.get()) },
                _invariant: PhantomData,
            })
        } else {
            // We can't unflip the UPGRADED bit back just yet as there is another
            // upgradeable or write lock. When they unlock, they will clear the
            // bit.
            None
        }
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for RwSpinLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_read() {
            Some(guard) => write!(f, "RwSpinLock {{ data: ")
                .and_then(|()| (&*guard).fmt(f))
                .and_then(|()| write!(f, "}}")),
            None => write!(f, "RwSpinLock {{ <locked> }}"),
        }
    }
}

impl<T: ?Sized + Default> Default for RwSpinLock<T> {
    fn default() -> RwSpinLock<T> {
        RwSpinLock::new(Default::default())
    }
}

impl<'rwlock, T: ?Sized> RwSpinLockUpgradeableGuard<'rwlock, T> {
    #[inline(always)]
    fn try_upgrade_internal(self, strong: bool) -> Result<RwSpinLockWriteGuard<'rwlock, T>, Self> {
        if compare_exchange(
            &self.lock,
            UPGRADED,
            WRITER,
            Ordering::Acquire,
            Ordering::Relaxed,
            strong,
        )
        .is_ok()
        {
            // Upgrade successful
            let out = Ok(RwSpinLockWriteGuard {
                lock: &self.lock,
                data: self.data,
                _invariant: PhantomData,
            });

            // Forget the old guard so its destructor doesn't run
            mem::forget(self);

            out
        } else {
            Err(self)
        }
    }

    #[inline]
    pub fn upgrade(mut self) -> RwSpinLockWriteGuard<'rwlock, T> {
        loop {
            self = match self.try_upgrade_internal(false) {
                Ok(guard) => return guard,
                Err(e) => e,
            };

            cpu_relax();
        }
    }

    #[inline]
    pub fn try_upgrade(self) -> Result<RwSpinLockWriteGuard<'rwlock, T>, Self> {
        self.try_upgrade_internal(true)
    }

    #[inline]
    pub fn downgrade(self) -> RwSpinLockReadGuard<'rwlock, T> {
        // Reserve the read guard for ourselves
        self.lock.fetch_add(READER, Ordering::Acquire);

        RwSpinLockReadGuard {
            lock: &self.lock,
            data: self.data,
        }

        // Dropping self removes the UPGRADED bit
    }
}

impl<'rwlock, T: ?Sized> RwSpinLockWriteGuard<'rwlock, T> {
    #[inline]
    pub fn downgrade(self) -> RwSpinLockReadGuard<'rwlock, T> {
        // Reserve the read guard for ourselves
        self.lock.fetch_add(READER, Ordering::Acquire);

        RwSpinLockReadGuard {
            lock: &self.lock,
            data: self.data,
        }

        // Dropping self removes the WRITER bit
    }
}

impl<'rwlock, T: ?Sized> Deref for RwSpinLockReadGuard<'rwlock, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.data.as_ref() }
    }
}

impl<'rwlock, T: ?Sized> Deref for RwSpinLockUpgradeableGuard<'rwlock, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.data.as_ref() }
    }
}

impl<'rwlock, T: ?Sized> Deref for RwSpinLockWriteGuard<'rwlock, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.data.as_ref() }
    }
}

impl<'rwlock, T: ?Sized> DerefMut for RwSpinLockWriteGuard<'rwlock, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.data.as_mut() }
    }
}

impl<'rwlock, T: ?Sized> Drop for RwSpinLockReadGuard<'rwlock, T> {
    fn drop(&mut self) {
        debug_assert!(self.lock.load(Ordering::Relaxed) & !(WRITER | UPGRADED) > 0);
        self.lock.fetch_sub(READER, Ordering::Release);
    }
}

impl<'rwlock, T: ?Sized> Drop for RwSpinLockUpgradeableGuard<'rwlock, T> {
    fn drop(&mut self) {
        debug_assert_eq!(
            self.lock.load(Ordering::Relaxed) & (WRITER | UPGRADED),
            UPGRADED
        );
        self.lock.fetch_sub(UPGRADED, Ordering::AcqRel);
    }
}

impl<'rwlock, T: ?Sized> Drop for RwSpinLockWriteGuard<'rwlock, T> {
    fn drop(&mut self) {
        debug_assert_eq!(self.lock.load(Ordering::Relaxed) & WRITER, WRITER);

        // Writer is responsible for clearing both WRITER and UPGRADED bits.
        // The UPGRADED bit may be set if an upgradeable lock attempts an upgrade while
        // this lock is held.
        self.lock.fetch_and(!(WRITER | UPGRADED), Ordering::Release);
    }
}

#[inline(always)]
fn compare_exchange(
    atomic: &AtomicUsize,
    current: usize,
    new: usize,
    success: Ordering,
    failure: Ordering,
    strong: bool,
) -> Result<usize, usize> {
    if strong {
        atomic.compare_exchange(current, new, success, failure)
    } else {
        atomic.compare_exchange_weak(current, new, success, failure)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Eq, PartialEq, Debug)]
    struct NonCopy(i32);

    test_case!(smoke, {
        let l = RwSpinLock::new(());
        drop(l.read());
        drop(l.write());
        drop((l.read(), l.read()));
        drop(l.write());
    });

    test_case!(rwlock_unsized, {
        let rw: &RwSpinLock<[i32]> = &RwSpinLock::new([1, 2, 3]);
        {
            let b = &mut *rw.write();
            b[0] = 4;
            b[2] = 5;
        }
        let comp: &[i32] = &[4, 2, 5];
        assert_eq!(&*rw.read(), comp);
    });

    test_case!(rwlock_try_write, {
        use core::mem::drop;

        let lock = RwSpinLock::new(0isize);
        let read_guard = lock.read();

        let write_result = lock.try_write();
        match write_result {
            None => (),
            Some(_) => assert!(
                false,
                "try_write should not succeed while read_guard is in scope"
            ),
        }

        drop(read_guard);
    });

    test_case!(rw_try_read, {
        let m = RwSpinLock::new(0);
        mem::forget(m.write());
        assert!(m.try_read().is_none());
    });

    test_case!(into_inner, {
        let m = RwSpinLock::new(NonCopy(10));
        assert_eq!(m.into_inner(), NonCopy(10));
    });

    test_case!(force_read_decrement, {
        let m = RwSpinLock::new(());
        mem::forget(m.read());
        mem::forget(m.read());
        mem::forget(m.read());
        assert!(m.try_write().is_none());
        unsafe {
            m.force_read_decrement();
            m.force_read_decrement();
        }
        assert!(m.try_write().is_none());
        unsafe {
            m.force_read_decrement();
        }
        assert!(m.try_write().is_some());
    });

    test_case!(force_write_unlock, {
        let m = RwSpinLock::new(());
        mem::forget(m.write());
        assert!(m.try_read().is_none());
        unsafe {
            m.force_write_unlock();
        }
        assert!(m.try_read().is_some());
    });

    test_case!(upgrade_downgrade, {
        let m = RwSpinLock::new(());
        {
            let _r = m.read();
            let upg = m.try_upgradeable_read().unwrap();
            assert!(m.try_read().is_none());
            assert!(m.try_write().is_none());
            assert!(upg.try_upgrade().is_err());
        }
        {
            let w = m.write();
            assert!(m.try_upgradeable_read().is_none());
            let _r = w.downgrade();
            assert!(m.try_upgradeable_read().is_some());
            assert!(m.try_read().is_some());
            assert!(m.try_write().is_none());
        }
        {
            let _u = m.upgradeable_read();
            assert!(m.try_upgradeable_read().is_none());
        }

        assert!(m.try_upgradeable_read().unwrap().try_upgrade().is_ok());
    });
}
