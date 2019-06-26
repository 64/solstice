use bootinfo::{MemoryRegion, MemoryRegionType};
use bootloader::bootinfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Region {
    addr: usize,
    size: usize,
}

// 64 is the number used in the bootloader crate
const MAX_REGIONS: usize = 64;
const ALLOC_SIZE: usize = 4096;
pub struct BumpAllocator {
    regions: [Region; MAX_REGIONS],
    offset: usize,
    region_count: usize,
    current_region: usize,
}

impl Region {
    pub fn new() -> Self {
        Self { addr: 0, size: 0 }
    }
}

impl BumpAllocator {
    pub fn new(mem_map: &[MemoryRegion]) -> Option<Self> {
        let mut bump = Self {
            regions: [Region::new(); MAX_REGIONS],
            offset: 0,
            region_count: 0,
            current_region: 0,
        };

        let mut index: usize = 0;
        for reg in mem_map {
            if reg.region_type == MemoryRegionType::Usable {
                bump.regions[index].addr = reg.range.start_addr() as usize;
                bump.regions[index].size =
                    reg.range.end_addr() as usize - reg.range.start_addr() as usize;
                bump.region_count += 1;
                index += 1;
            }
        }

        // index will be 0 if no usable memory region is found
        if index == 0 {
            return None;
        } else {
            return Some(bump);
        }
    }

    pub fn alloc_page(&mut self) -> Option<usize> {
        while self.regions[self.current_region].size < ALLOC_SIZE {
            self.current_region += 1;
            if self.current_region >= self.region_count {
                return None;
            }
        }

        let addr = self.regions[self.current_region].addr;
        self.regions[self.current_region].addr += ALLOC_SIZE;
        self.regions[self.current_region].size -= ALLOC_SIZE;

        return Some(addr);
    }
}
