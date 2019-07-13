pub const PHYS_OFFSET: u64 = 0xFFFF8000_00000000;

pub mod addr_space;
pub mod bump;
pub mod pmm;
pub mod convert;

pub use convert::{to_phys, to_virt};
