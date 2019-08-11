pub const PHYS_OFFSET: usize = 0xFFFF8000_00000000;
pub const PAGE_SIZE: usize = 0x1000;

pub mod addr_space;
pub mod map;
pub mod pmm;
pub mod slob;
