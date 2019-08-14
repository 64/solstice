pub const PHYS_OFFSET: usize = 0xFFFF8000_00000000;
pub const PAGE_INFO_OFFSET: usize = 0xFFFF9000_00000000;
pub const PAGE_SIZE: usize = 0x1000;

use crate::ds::RwSpinLock;
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
    let idx = frame.start_address().as_usize() / PAGE_SIZE;
    let out_addr = PAGE_INFO_OFFSET + idx * core::mem::size_of::<RwSpinLock<PageInfo>>();

    // Check that it's not too large
    debug_assert!(out_addr < PAGE_INFO_OFFSET + 0x0000100000000000);

    out_addr as *const PageInfo
}
