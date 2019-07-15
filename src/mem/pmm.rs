use crate::mem::map::{MemoryMap, RegionBumpAllocator};
use arrayvec::ArrayVec;
use core::{alloc::Layout, mem, num::NonZeroU8, slice};
use x86_64::VirtAddr;

pub const MAX_ZONES: usize = 64;

#[derive(Debug)]
struct Zone {
    addr: VirtAddr,
    size: usize,
    blocks: &'static mut [Block],
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Block {
    LargestFreeOrder(NonZeroU8),
    Used,
}

impl Block {
    fn order(largest_free_order: u8) -> Self {
        Block::LargestFreeOrder(unsafe { NonZeroU8::new_unchecked(largest_free_order + 1) })
    }

    fn new_blocks_for_region(
        region: &mut RegionBumpAllocator,
        usable_pages: usize,
    ) -> &'static mut [Block] {
        let ptr = region
            .alloc(Layout::from_size_align(usable_pages * 2, 1).unwrap())
            .expect("failed to allocate from region");
        debug_assert_eq!(
            ptr.as_ptr() as usize,
            x86_64::align_down(ptr.as_ptr() as usize, super::PAGE_SIZE)
        );

        unsafe { 
            // Zero out the memory, which corresponds to Block::Used
            core::intrinsics::write_bytes(ptr.as_ptr(), 0, usable_pages * 2);
            slice::from_raw_parts_mut(ptr.as_ptr() as *mut Block, usable_pages * 2)
        }
    }
}

struct PageInfo;

pub struct PhysAllocator {
    zones: ArrayVec<[Zone; MAX_ZONES]>,
}

impl PhysAllocator {
    pub fn new(map: MemoryMap) -> Self {
        let mut zones = ArrayVec::new();

        for rg in map {
            let pages_in_rg = rg.size / super::PAGE_SIZE;
            let usable_pages = usable_pages(pages_in_rg);
            let (reserved, usable) = rg.split_at((pages_in_rg - usable_pages) * super::PAGE_SIZE);

            let mut reserved_allocator = RegionBumpAllocator::from(reserved);

            zones.push(Zone {
                addr: usable.addr.into(),
                size: x86_64::align_down(usable.size, super::PAGE_SIZE),
                blocks: Block::new_blocks_for_region(&mut reserved_allocator, usable_pages),
            });

            assert_eq!(usable.addr.as_usize() & (super::PAGE_SIZE - 1), 0); // Make sure it's aligned
        }

        Self { zones }
    }
}

// Each page of memory has a constant memory overhead of 2 * size_of::<Block>()
// + size_of::<PageInfo>() Let N = number of (PMM) usable memory
//     T = total number of pages, usable and unusable
//     W = overhead per page in bytes
// We have the equation
// total wasted bytes <= 4096 * (T - N)
//              N * W <= 4096T - 4096N
//     N * (W + 4096) <= 4096T
//              N - 1 < 4096T / (W + 4096)         (due to integer division
// truncation) Hence: Max usable N = 4096T / (W + 4096) - 1
// Subtract one extra page, just to be safe about padding and alignment
fn usable_pages(total_pages: usize) -> usize {
    4096 * (total_pages as usize)
        / (2 * mem::size_of::<Block>() + mem::size_of::<PageInfo>() + 4096)
        - 2
}

#[cfg(test)]
mod tests {
    use super::*;

    test_case!(block_repr, {
        let b: u8 = 0;
        let block = &b as *const u8 as *const Block;
        assert_eq!(mem::size_of::<Block>(), 1);
        assert_eq!(mem::align_of::<Block>(), 1);
        assert_eq!(unsafe { *block }, Block::Used);
    });
}
