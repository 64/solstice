use core::slice;

use bootloader::bootinfo::{E820MemoryRegion, MemoryMap, MemoryRegion, MemoryRegionType};
use x86_64::VirtAddr;

pub(crate) fn create_from(memory_map_addr: VirtAddr, entry_count: usize) -> MemoryMap {
    let memory_map_start_ptr = memory_map_addr.as_usize() as *const E820MemoryRegion;
    let e820_memory_map =
        unsafe { slice::from_raw_parts(memory_map_start_ptr, entry_count) };

    let mut memory_map = MemoryMap::new();
    for region in e820_memory_map {
        memory_map.add_region(MemoryRegion::from(*region));
    }

    memory_map.sort();

    let mut iter = memory_map.iter_mut().peekable();
    while let Some(region) = iter.next() {
        if let Some(next) = iter.peek() {
            if region.range.end_frame_number > next.range.start_frame_number
                && region.region_type == MemoryRegionType::Usable
            {
                region.range.end_frame_number = next.range.start_frame_number;
            }
        }
    }

    memory_map
}
