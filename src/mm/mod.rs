pub const PHYS_OFFSET: u64 = 0xFFFF8000_00000000;
pub const PAGE_INFO_OFFSET: u64 = 0xFFFF9000_00000000;
pub const PAGE_SIZE: u64 = 0x1000;

use crate::ds::RwSpinLock;
use x86_64::{VirtAddr, PhysAddr};
use x86_64::structures::paging::PhysFrame;

pub mod addr_space;
pub mod map;
pub mod pmm;
pub mod slob;

#[derive(Default)]
pub struct PageInfo {
    _dummy: i64,
}

pub fn phys_to_page_info(frame: PhysFrame) -> *const PageInfo {
    let idx = frame.start_address().as_u64() / PAGE_SIZE;
    let out_addr = PAGE_INFO_OFFSET + idx * (core::mem::size_of::<RwSpinLock<PageInfo>>()) as u64;

    // Check that it's not too large
    debug_assert!(out_addr < PAGE_INFO_OFFSET + 0x0000100000000000);

    out_addr as *const PageInfo
}

pub fn kernel_virt_to_phys(virt: VirtAddr) -> PhysAddr {
    debug_assert!(virt.as_u64() >= PHYS_OFFSET);
    PhysAddr::new(virt.as_u64() - PHYS_OFFSET)
}

pub fn phys_to_kernel_virt(phys: PhysAddr) -> VirtAddr {
    VirtAddr::new(phys.as_u64() + PHYS_OFFSET)
}
