use arrayvec::ArrayVec;
use bootloader::bootinfo::{MemoryRegion, MemoryRegionType};
use core::{alloc::Layout, ptr::NonNull};
use x86_64::{
    structures::paging::{PageSize, PhysFrame, Size4KiB},
    PhysAddr,
    VirtAddr,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub addr: PhysAddr,
    pub size: usize,
}

impl Region {
    pub fn split_at(self, offset: usize) -> (Region, Region) {
        assert!(offset < self.size);
        (
            Region {
                addr: self.addr,
                size: offset,
            },
            Region {
                addr: PhysAddr::new(self.addr.as_usize() + offset),
                size: self.size - offset,
            },
        )
    }
}

// 64 is the number used in the bootloader crate
const MAX_REGIONS: usize = 64;

// TODO: Reference the memory map from bootloader crate instead
#[derive(Debug, Clone, Default)]
pub struct MemoryMap {
    regions: ArrayVec<[Region; MAX_REGIONS]>,
    pub num_pages: usize,
}

impl MemoryMap {
    pub fn new(memory_map: &[MemoryRegion]) -> Self {
        let mut bump = Self {
            regions: ArrayVec::new(),
            num_pages: 0,
        };

        for reg in memory_map.iter() {
            if reg.region_type == MemoryRegionType::Usable {
                bump.push(Region {
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

    fn push(&mut self, rg: Region) {
        self.num_pages += rg.size / Size4KiB::SIZE;
        self.regions.push(rg);
    }

    #[allow(unused)]
    fn alloc_page(&mut self) -> PhysFrame {
        let (idx, found_region) = self
            .regions
            .iter_mut()
            .enumerate()
            .find(|(_, rg)| rg.size >= Size4KiB::SIZE)
            .expect("bump allocator - out of memory");

        let out = PhysFrame::containing_address(found_region.addr);

        found_region.addr += Size4KiB::SIZE;
        found_region.size -= Size4KiB::SIZE;
        self.num_pages -= 1;

        if found_region.size == 0 {
            // Can't allocate from this region anymore
            self.regions.remove(idx);
        }

        // Clear the page
        #[cfg(not(test))]
        unsafe {
            let page: *mut u8 = VirtAddr::from(out.start_address()).as_mut_ptr();
            core::intrinsics::write_bytes(
                page,
                if cfg!(debug_assertions) { 0xB8 } else { 0x00 },
                Size4KiB::SIZE,
            )
        };

        out
    }
}

impl IntoIterator for MemoryMap {
    type Item = Region;
    type IntoIter = RegionIter;

    fn into_iter(self) -> Self::IntoIter {
        RegionIter {
            regions: self.regions,
        }
    }
}

pub struct RegionIter {
    regions: ArrayVec<[Region; MAX_REGIONS]>,
}

impl Iterator for RegionIter {
    type Item = Region;

    fn next(&mut self) -> Option<Self::Item> {
        self.regions.pop_at(0)
    }
}

// Allocates from a physically contiguous chunk of memory
pub struct RegionBumpAllocator {
    start: PhysAddr,
    size: usize,
    offset: usize,
}

impl RegionBumpAllocator {
    pub fn alloc(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let new_off = x86_64::align_up(self.offset + layout.size(), layout.align());

        if new_off > self.size {
            None
        } else {
            let out = NonNull::new(
                VirtAddr::new(
                    self.start.as_usize()
                        + x86_64::align_up(self.offset, layout.align())
                        + super::PHYS_OFFSET,
                )
                .as_mut_ptr(),
            )
            .unwrap();
            self.offset = new_off;
            Some(out)
        }
    }
}

impl From<Region> for RegionBumpAllocator {
    fn from(rg: Region) -> Self {
        Self {
            start: rg.addr,
            size: rg.size,
            offset: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    test_case!(allocate, {
        use bootloader::bootinfo::FrameRange;

        let mut bump = MemoryMap::new(&[
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

        let a = |addr: usize| PhysFrame::containing_address(PhysAddr::new(addr));

        assert_eq!(bump.num_pages, 3);
        assert_eq!(bump.alloc_page(), a(0x1000));
        assert_eq!(bump.num_pages, 2);
        assert_eq!(bump.alloc_page(), a(0x3000));
        assert_eq!(bump.num_pages, 1);
        assert_eq!(bump.alloc_page(), a(0x4000));
        assert_eq!(bump.num_pages, 0);
    });

    test_case!(region, {
        // Bump allocation
        let mut rg_bump = RegionBumpAllocator::from(Region {
            addr: PhysAddr::new(0x1000),
            size: 4096,
        });
        assert_eq!(
            rg_bump.alloc(Layout::from_size_align(4, 4).unwrap()),
            Some(NonNull::new((crate::mem::PHYS_OFFSET + 0x1000) as *mut _).unwrap())
        );
        assert_eq!(
            rg_bump.alloc(Layout::from_size_align(1, 1).unwrap()),
            Some(NonNull::new((crate::mem::PHYS_OFFSET + 0x1004) as *mut _).unwrap())
        );
        assert_eq!(
            rg_bump.alloc(Layout::from_size_align(4, 4).unwrap()),
            Some(NonNull::new((crate::mem::PHYS_OFFSET + 0x1008) as *mut _).unwrap())
        );
        assert_eq!(
            rg_bump.alloc(Layout::from_size_align(4096, 4).unwrap()),
            None
        );

        // Splitting
        assert_eq!(
            Region {
                addr: PhysAddr::new(0x1000),
                size: 4096,
            }
            .split_at(100),
            (
                Region {
                    addr: PhysAddr::new(0x1000),
                    size: 100,
                },
                Region {
                    addr: PhysAddr::new(0x1000 + 100),
                    size: 4096 - 100,
                }
            )
        );
    });
}
