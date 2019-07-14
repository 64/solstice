pub const PHYS_OFFSET: usize = 0xFFFF8000_00000000;

pub mod addr_space;
pub mod convert;
pub mod map;
pub mod pmm;

pub use convert::{to_phys, to_virt};
