use crate::mem::addr_space::AddrSpace;
use arrayvec::ArrayVec;
use core::{
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct PerCpu {
    addr_space: *const AddrSpace,
    preempt_count: AtomicUsize,
}

unsafe impl Send for PerCpu {}
unsafe impl Sync for PerCpu {}

const MAX_CPUS: usize = 8;

lazy_static! {
    pub static ref CPUS: ArrayVec<[PerCpu; MAX_CPUS]> = {
        let mut cpus = ArrayVec::new();

        cpus.push(PerCpu {
            addr_space: AddrSpace::kernel(),
            preempt_count: AtomicUsize::new(0),
        });

        cpus
    };
}

impl PerCpu {
    pub fn current() -> &'static PerCpu {
        &CPUS[0] // TODO: SMP
    }

    pub unsafe fn preempt_inc(&self) {
        self.preempt_count.fetch_add(1, Ordering::Acquire);
    }

    pub unsafe fn preempt_dec(&self) {
        self.preempt_count.fetch_sub(1, Ordering::Acquire);
    }

    pub fn without_preempts<T, F>(f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let current = PerCpu::current();

        unsafe { current.preempt_inc() };
        let rv = f();
        unsafe { current.preempt_dec() };

        rv
    }

    #[cfg(test)]
    pub fn preempt_count(&self, ordering: Ordering) -> usize {
        self.preempt_count.load(ordering)
    }
}
