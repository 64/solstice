use bootloader::bootinfo::{MemoryRegion, MemoryRegionType};
use x86_64::{
    structures::paging::{PhysFrame, Size4KiB},
    PhysAddr,
};

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
    region_count: usize,
    current_region: usize,
}

// Align address up to next ALLOX_SIZE(4096) page boundry
fn alignup(val: usize) -> usize {
    ((val + (ALLOC_SIZE - 1)) as isize & -(ALLOC_SIZE as isize)) as usize
}

impl Region {
    pub fn new() -> Self {
        Self { addr: 0, size: 0 }
    }
}

impl BumpAllocator {
    pub fn new(mem_map: &[MemoryRegion]) -> Self {
        let mut bump = Self {
            regions: [Region::new(); MAX_REGIONS],
            region_count: 0,
            current_region: 0,
        };

        for reg in mem_map {
            if reg.region_type == MemoryRegionType::Usable {
                let addr = alignup(reg.range.start_addr() as usize);
                bump.regions[bump.region_count].addr = addr;
                bump.regions[bump.region_count].size = reg.range.end_addr() as usize - addr;
                bump.region_count += 1;
            }
        }

        // index will be 0 if no usable memory region is found
        if bump.region_count == 0 {
            panic!("No physical usable memory region found");
        } else {
            bump
        }
    }

    pub fn alloc_page(&mut self) -> PhysFrame<Size4KiB> {
        while self.regions[self.current_region].size < ALLOC_SIZE {
            self.current_region += 1;
            if self.current_region >= self.region_count {
                panic!("Unable to allocate physical frame");
            }
        }

        let addr = self.regions[self.current_region].addr as u64;
        self.regions[self.current_region].addr += ALLOC_SIZE;
        self.regions[self.current_region].size -= ALLOC_SIZE;

        PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(addr))
    }
}
