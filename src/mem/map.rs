use arrayvec::ArrayVec;
use bootloader::bootinfo::{BootInfo, MemoryRegion, MemoryRegionType};
use core::{alloc::Layout, ptr::NonNull, slice};
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
                addr: PhysAddr::new(self.addr.as_u64() + offset as u64),
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
                    size: (reg.range.end_addr() - reg.range.start_addr()) as usize,
                });
            }
        }

        if bump.regions.len() == 0 {
            panic!("no physical usable memory regions found");
        }

        bump
    }

    fn push(&mut self, rg: Region) {
        self.num_pages += rg.size / Size4KiB::SIZE as usize;
        self.regions.push(rg);
    }

    pub fn split_at(self, num_pages: usize) -> (Self, Self) {
        assert_ne!(num_pages, 0);

        let mut first = MemoryMap::default();
        let mut second = MemoryMap::default();

        let mut pages_seen = 0;
        let mut copy_from = None;

        for (i, rg) in self.regions.iter().enumerate() {
            let pages_in_region = rg.size / Size4KiB::SIZE as usize;
            pages_seen += pages_in_region;

            if pages_seen > num_pages {
                // Need to split this region
                let start_addr = rg.addr.as_u64() as usize;
                let end_addr = start_addr
                    + Size4KiB::SIZE as usize * (num_pages + pages_in_region - pages_seen);
                first.push(Region {
                    addr: PhysAddr::new(start_addr as u64),
                    size: end_addr - start_addr,
                });

                second.push(Region {
                    addr: PhysAddr::new(end_addr as u64),
                    size: start_addr + pages_in_region * Size4KiB::SIZE as usize as usize
                        - end_addr,
                });

                copy_from = Some(i + 1);
                break;
            } else {
                // Can take this entire region, no splitting
                first.push(*rg);

                if pages_seen == num_pages {
                    copy_from = Some(i + 1);
                    break;
                }
            }
        }

        // Bounds check
        if let Some(copy_from) = copy_from.filter(|&c| c < self.regions.len()) {
            for rg in &self.regions[copy_from..] {
                second.push(Region {
                    addr: rg.addr,
                    size: rg.size,
                });
            }
        }

        (first, second)
    }

    pub fn alloc_page(&mut self) -> PhysFrame {
        let (idx, found_region) = self
            .regions
            .iter_mut()
            .enumerate()
            .find(|(_, rg)| rg.size >= Size4KiB::SIZE as usize)
            .expect("bump allocator - out of memory");

        let out = PhysFrame::containing_address(found_region.addr);

        found_region.addr += Size4KiB::SIZE as usize;
        found_region.size -= Size4KiB::SIZE as usize;
        self.num_pages -= 1;

        if found_region.size == 0 {
            // Can't allocate from this region anymore
            self.regions.remove(idx);
        }

        // Clear the page
        #[cfg(not(test))]
        unsafe {
            let page: *mut u8 = super::to_virt(out.start_address()).as_mut_ptr();
            core::intrinsics::write_bytes(
                page,
                if cfg!(debug_assertions) { 0xB8 } else { 0x00 },
                Size4KiB::SIZE as usize,
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
        let new_off =
            x86_64::align_up((self.offset + layout.size()) as u64, layout.align() as u64) as usize;

        if new_off > self.size {
            None
        } else {
            let out = NonNull::new(
                VirtAddr::new(
                    self.start.as_u64()
                        + x86_64::align_up(self.offset as u64, layout.align() as u64)
                        + super::PHYS_OFFSET as u64,
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
            size: rg.size as usize,
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

        let a = |addr: u64| PhysFrame::containing_address(PhysAddr::new(addr));

        assert_eq!(bump.num_pages, 3);
        assert_eq!(bump.alloc_page(), a(0x1000));
        assert_eq!(bump.num_pages, 2);
        assert_eq!(bump.alloc_page(), a(0x3000));
        assert_eq!(bump.num_pages, 1);
        assert_eq!(bump.alloc_page(), a(0x4000));
        assert_eq!(bump.num_pages, 0);

        // Test region allocation
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
    });

    test_case!(map_split, {
        use bootloader::bootinfo::FrameRange;

        let bump = MemoryMap::new(&[
            MemoryRegion {
                range: FrameRange::new(0x1000, 0x5000),
                region_type: MemoryRegionType::Usable,
            },
            MemoryRegion {
                range: FrameRange::new(0x6000, 0x8000),
                region_type: MemoryRegionType::Usable,
            },
            MemoryRegion {
                range: FrameRange::new(0x9000, 0xA000),
                region_type: MemoryRegionType::Usable,
            },
        ]);
        let bump2 = bump.clone();

        let a = |addr: u64| PhysFrame::containing_address(PhysAddr::new(addr));

        {
            let (mut left, mut right) = bump.split_at(5);
            assert_eq!(left.num_pages, 5);
            assert_eq!(right.num_pages, 2);
            assert_eq!(left.alloc_page(), a(0x1000));
            assert_eq!(left.alloc_page(), a(0x2000));
            assert_eq!(left.alloc_page(), a(0x3000));
            assert_eq!(left.alloc_page(), a(0x4000));
            assert_eq!(left.alloc_page(), a(0x6000));
            assert_eq!(right.alloc_page(), a(0x7000));
            assert_eq!(right.alloc_page(), a(0x9000));
        }

        {
            let (mut left, mut right) = bump2.split_at(4);
            assert_eq!(left.num_pages, 4);
            assert_eq!(right.num_pages, 3);
            assert_eq!(left.alloc_page(), a(0x1000));
            assert_eq!(left.alloc_page(), a(0x2000));
            assert_eq!(left.alloc_page(), a(0x3000));
            assert_eq!(left.alloc_page(), a(0x4000));
            assert_eq!(right.alloc_page(), a(0x6000));
            assert_eq!(right.alloc_page(), a(0x7000));
            assert_eq!(right.alloc_page(), a(0x9000));
        }

        // Test region splitting
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
