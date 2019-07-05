use crate::ds::RwSpinLock;
use core::ptr::NonNull;
use x86_64::structures::paging::PageTable;

pub struct AddrSpace {
    table: RwSpinLock<NonNull<PageTable>>,
}
