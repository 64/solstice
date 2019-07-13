use crate::ds::RwSpinLock;
use core::ptr::NonNull;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        frame::PhysFrame,
        mapper::{MapToError, MapperFlush},
        page::Size4KiB,
        FrameAllocator,
        Mapper,
        OffsetPageTable,
        Page,
        PageTable,
        PageTableFlags,
    },
    PhysAddr,
    VirtAddr,
};

pub struct AddrSpace {
    table: RwSpinLock<OffsetPageTable<'static>>,
}

unsafe impl Send for AddrSpace {}
unsafe impl Sync for AddrSpace {}

lazy_static! {
    static ref KERNEL: AddrSpace = {
        let (table_frame, _) = Cr3::read();
        let table_virt = super::to_virt(table_frame.start_address());

        AddrSpace {
            table: RwSpinLock::new(unsafe {
                OffsetPageTable::new(&mut *table_virt.as_mut_ptr(), super::PHYS_OFFSET)
            }),
        }
    };
}

impl AddrSpace {
    pub fn kernel() -> &'static AddrSpace {
        &*KERNEL
    }

    // TODO: Make sure that allocations and deallocations are done with the same allocator?
    pub fn map_to_with_allocator<A: FrameAllocator<Size4KiB>>(
        &self,
        virt: VirtAddr,
        phys: PhysAddr,
        flags: PageTableFlags,
        alloc: &mut A,
    ) -> Result<MapperFlush<Size4KiB>, MapToError> {
        unsafe {
            self.table.write().map_to(
                Page::containing_address(virt),
                PhysFrame::containing_address(phys),
                flags,
                alloc,
            )
        }
    }
}
