use crate::ds::RwSpinLock;
use core::ptr::NonNull;
use x86_64::{registers::control::Cr3, structures::paging::PageTable};

pub struct AddrSpace {
    table: RwSpinLock<NonNull<PageTable>>,
}

unsafe impl Send for AddrSpace {}
unsafe impl Sync for AddrSpace {}

lazy_static! {
    static ref KERNEL: AddrSpace = {
        let (table_frame, _) = Cr3::read();
        let table_virt = table_frame.start_address().as_u64() + 0xFFFF800000000000; // TODO
        let ptr = NonNull::new(table_virt as *mut _).unwrap();

        AddrSpace {
            table: RwSpinLock::new(ptr),
        }
    };
}

impl AddrSpace {
    pub fn kernel() -> *const AddrSpace {
        &*KERNEL
    }
}
