use arrayvec::ArrayVec;
use bootloader::bootinfo::{MemoryRegion, MemoryRegionType};
use x86_64::{
    structures::paging::{PhysFrame, Size4KiB},
    PhysAddr,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Region {
    addr: PhysAddr,
    size: usize,
}

// 64 is the number used in the bootloader crate
const MAX_REGIONS: usize = 64;

#[derive(Debug)]
pub struct BumpAllocator {
    regions: ArrayVec<[Region; MAX_REGIONS]>,
}

impl BumpAllocator {
    pub fn new(mem_map: &[MemoryRegion]) -> Self {
        let mut bump = Self {
            regions: ArrayVec::new(),
        };

        for reg in mem_map {
            if reg.region_type == MemoryRegionType::Usable {
                bump.regions.push(Region {
                    addr: PhysAddr::new(reg.range.start_addr()),
                    size: reg.range.end_addr() as usize - reg.range.start_addr() as usize,
                });
            }
        }

        if bump.regions.len() == 0 {
            panic!("no physical usable memory regions found");
        }

        bump
    }

    pub fn alloc_page(&mut self) -> PhysFrame {
        const ALLOC_SIZE: usize = 4096;

        let (idx, found_region) = self
            .regions
            .iter_mut()
            .enumerate()
            .find(|(_, rg)| rg.size >= ALLOC_SIZE)
            .expect("bump allocator - out of memory");

        let out_addr = found_region.addr;

        found_region.addr += ALLOC_SIZE;
        found_region.size -= ALLOC_SIZE;

        // Can't allocate from this region anymore
        if found_region.size == 0 {
            self.regions.remove(idx);
        }

        PhysFrame::<Size4KiB>::containing_address(out_addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    test_case!(allocate, {
        use bootloader::bootinfo::FrameRange;

        let mut bump = BumpAllocator::new(&[
            MemoryRegion {
                range: FrameRange::new(0x1000, 0x2000),
                region_type: MemoryRegionType::Usable,
            },
            MemoryRegion {
                range: FrameRange::new(0x2000, 0x3000),
                region_type: MemoryRegionType::Reserved,
            },
            MemoryRegion {
                range: FrameRange::new(0x3000, 0x5000),
                region_type: MemoryRegionType::Usable,
            },
        ]);

        let a = |addr: u64| PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(addr));

        assert_eq!(bump.alloc_page(), a(0x1000));
        assert_eq!(bump.alloc_page(), a(0x3000));
        assert_eq!(bump.alloc_page(), a(0x4000));
    });
}
