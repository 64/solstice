use crate::mem::map::{MemoryMap, RegionBumpAllocator};
use arrayvec::ArrayVec;
use core::{mem, num::NonZeroU8};
use x86_64::VirtAddr;

pub const MAX_ZONES: usize = 64;

struct Zone {
    addr: VirtAddr,
    size: usize,
    blocks: &'static mut [Block],
}

enum Block {
    LargestFreeOrder(NonZeroU8),
    Used,
}

impl Block {
    fn order(largest_free_order: u8) -> Self {
        Block::LargestFreeOrder(unsafe { NonZeroU8::new_unchecked(largest_free_order + 1) })
    }

    // fn new_array(bump: &mut BumpAllocator, pages: usize) -> &'static mut [Block]
    // {     let alloc_size = bump.alloc_sub_page(mem::size_of::<Block>(),
    // mem::align_of::<Block>()) }
}

struct PageInfo;

pub struct PhysAllocator {
    zones: ArrayVec<[Zone; MAX_ZONES]>,
}

impl PhysAllocator {
    pub fn new(map: MemoryMap) -> Self {
        let mut zones = ArrayVec::new();

        let usable_pages = usable_pages(map.num_pages);
        let reserved_pages = map.num_pages - usable_pages;

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

    test_case!(size_align, {
        assert_eq!(mem::size_of::<Block>(), 1);
        assert_eq!(mem::align_of::<Block>(), 1);
    });
}
