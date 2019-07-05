use crate::ds::RwSpinLock;
use x86_64::structures::paging::PageTable;
use core::ptr::NonNull;

pub struct AddrSpace {
    table: RwSpinLock<NonNull<PageTable>>,     
}
