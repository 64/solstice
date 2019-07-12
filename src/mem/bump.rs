use arrayvec::ArrayVec;
use bootloader::bootinfo::{MemoryRegion, MemoryRegionType};
use core::slice;
use x86_64::{
    structures::paging::{PageSize, PhysFrame, Size4KiB},
    PhysAddr,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub addr: PhysAddr,
    pub size: u64,
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
                    size: reg.range.end_addr() - reg.range.start_addr(),
                });
            }
        }

        if bump.regions.len() == 0 {
            panic!("no physical usable memory regions found");
        }

        bump
    }

    pub fn alloc_page(&mut self) -> PhysFrame {
        let (idx, found_region) = self
            .regions
            .iter_mut()
            .enumerate()
            .find(|(_, rg)| rg.size >= Size4KiB::SIZE)
            .expect("bump allocator - out of memory");

        let out = PhysFrame::containing_address(found_region.addr);

        found_region.addr += Size4KiB::SIZE;
        found_region.size -= Size4KiB::SIZE;

        if found_region.size == 0 {
            // Can't allocate from this region anymore
            self.regions.remove(idx);
        }

        out
    }
}

impl<'a> IntoIterator for &'a BumpAllocator {
    type Item = &'a Region;
    type IntoIter = slice::Iter<'a, Region>;

    fn into_iter(self) -> Self::IntoIter {
        self.regions.iter()
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

        let a = |addr: u64| PhysFrame::containing_address(PhysAddr::new(addr));

        assert_eq!(bump.alloc_page(), a(0x1000));
        assert_eq!(bump.alloc_page(), a(0x3000));
        assert_eq!(bump.alloc_page(), a(0x4000));
    });
}
