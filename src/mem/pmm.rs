use crate::{
    ds::SpinLock,
    mem::map::{MemoryMap, Region, RegionBumpAllocator},
};
use arrayvec::ArrayVec;
use core::{alloc::Layout, mem, num::NonZeroU8, slice};
use x86_64::{
    structures::paging::frame::{PhysFrame, PhysFrameRange},
    PhysAddr,
};

pub const MAX_ZONES: usize = 64;
pub const MAX_ORDER: usize = 11;
pub const MAX_ORDER_PAGES: usize = 1 << 11;

#[derive(Debug)]
struct Zone {
    pages: PhysFrameRange,
    num_pages: usize,
    order_list: [&'static mut [Block]; MAX_ORDER + 1],
}

impl Zone {
    pub fn new(addr: PhysAddr, size: usize, blocks: &'static mut [Block]) -> Self {
        let num_pages = size / super::PAGE_SIZE;

        let mut order_list = Self::split_region(num_pages, blocks);

        let mut blocks_in_order = num_pages;
        for (order, list) in order_list.iter_mut().enumerate() {
            for block in list.iter_mut().take(blocks_in_order) {
                *block = Block::from_order(order as u8);
            }

            blocks_in_order = blocks_in_order / 2 + if blocks_in_order % 2 == 0 { 0 } else { 1 };
        }

        let largest_order =
            (num_pages.next_power_of_two().trailing_zeros() as usize).min(MAX_ORDER + 1);
        for list in order_list[largest_order..].iter_mut() {
            list[0] = Block::from_order(largest_order as u8);
        }

        let start_frame = PhysFrame::containing_address(addr);
        let end_frame = start_frame + num_pages;

        Zone {
            pages: PhysFrame::range(start_frame, end_frame),
            num_pages,
            order_list,
        }
    }

    fn split_region(
        num_pages: usize,
        mut blocks: &'static mut [Block],
    ) -> [&'static mut [Block]; MAX_ORDER + 1] {
        let max_order_blocks = x86_64::align_up(num_pages, MAX_ORDER_PAGES) / MAX_ORDER_PAGES;

        // TODO: This whole section is a bit of a hack
        let mut tmp: [Option<&'static mut [Block]>; MAX_ORDER + 1] = [
            None, None, None, None, None, None, None, None, None, None, None, None,
        ];

        for (order, block_slice) in tmp.iter_mut().rev().enumerate() {
            let blocks_in_layer = max_order_blocks * 2_usize.pow(order as u32);
            let (left, right) = blocks.split_at_mut(blocks_in_layer);
            *block_slice = Some(left);
            blocks = right;
        }

        unsafe { core::mem::transmute(tmp) }
    }

    // Iterate back up, setting parents to have the correct largest order value
    fn update_tree(&mut self, start_order: u8, mut idx: usize) {
        for current_order in start_order + 1..=MAX_ORDER as u8 {
            let left_idx = idx & !1;
            let left = self.order_list[current_order as usize - 1][left_idx];
            let right = self.order_list[current_order as usize - 1][left_idx + 1];
            self.order_list[current_order as usize][idx / 2] = Block::parent_state(left, right);
            idx /= 2;
        }
    }

    fn alloc(&mut self, order: u8) -> Option<PhysFrameRange> {
        // TODO: This can be optimised quite a bit (use linked lists?)
        // Find top level index
        let mut idx = self.order_list[MAX_ORDER]
            .iter()
            .enumerate()
            .find(|(_, blk)| blk.larger_than(order))?
            .0;

        for current_order in (order..(MAX_ORDER as u8)).rev() {
            idx *= 2;

            idx = if self.order_list[current_order as usize][idx].larger_than(order) {
                idx
            } else if self.order_list[current_order as usize][idx + 1].larger_than(order) {
                idx + 1
            } else {
                unreachable!();
            };
        }

        self.order_list[order as usize][idx] = Block::Used;
        self.update_tree(order, idx);

        let start_frame = self.pages.start + 2usize.pow(order as u32) * idx;
        let end_frame = self.pages.start + 2usize.pow(order as u32) * (idx + 1);
        Some(PhysFrame::range(start_frame, end_frame))
    }

    fn free(&mut self, range: PhysFrameRange) {
        let order = range.len().trailing_zeros();
        debug_assert!(order as usize <= MAX_ORDER);
        debug_assert!(self.pages.contains_range(range));

        let idx = (range.start - self.pages.start) / range.len();
        debug_assert_eq!(self.order_list[order as usize][idx], Block::Used);

        self.order_list[order as usize][idx] = Block::from_order(order as u8);
        self.update_tree(order as u8, idx);
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Block {
    LargestFreeOrder(NonZeroU8),
    Used,
}

impl core::fmt::Debug for Block {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Block::LargestFreeOrder(nzu) => {
                fmt.write_fmt(format_args!("LargestFreeOrder({})", nzu.get() - 1))
            }
            Block::Used => fmt.write_str("Used"),
        }
    }
}

impl Block {
    fn from_order(largest_free_order: u8) -> Self {
        Block::LargestFreeOrder(unsafe { NonZeroU8::new_unchecked(largest_free_order + 1) })
    }

    fn larger_than(self, order: u8) -> bool {
        match self {
            // This is really a 'greater than or equal', since o.get() is one larger than the page
            // it indicates
            Block::LargestFreeOrder(o) => o.get() > order,
            _ => false,
        }
    }

    fn parent_state(left: Self, right: Self) -> Self {
        match (left, right) {
            (Block::LargestFreeOrder(l), Block::LargestFreeOrder(r)) => {
                let order = if l == r {
                    unsafe { NonZeroU8::new_unchecked(l.get() + 1) }
                } else if l > r {
                    l
                } else {
                    r
                };

                Block::LargestFreeOrder(order)
            }
            (Block::LargestFreeOrder(x), _) | (_, Block::LargestFreeOrder(x)) => {
                Block::LargestFreeOrder(x)
            }
            _ => Block::Used,
        }
    }

    fn new_blocks_for_region(region: Region, usable_pages: usize) -> &'static mut [Block] {
        let block_count = blocks_in_region(usable_pages);

        let mut rg_allocator = RegionBumpAllocator::from(region);
        let ptr = rg_allocator
            .alloc(
                Layout::from_size_align(
                    block_count * mem::size_of::<Block>(),
                    mem::align_of::<Block>(),
                )
                .unwrap(),
            )
            .expect("failed to allocate from region");

        debug_assert_eq!(
            ptr.as_ptr() as usize,
            x86_64::align_down(ptr.as_ptr() as usize, super::PAGE_SIZE)
        );

        unsafe {
            // Zero out the memory, which corresponds to Block::Used
            core::intrinsics::write_bytes(ptr.as_ptr(), 0, block_count);
            slice::from_raw_parts_mut(ptr.as_ptr() as *mut Block, block_count)
        }
    }
}

struct PageInfo;

pub struct PhysAllocator {
    zones: ArrayVec<[SpinLock<Zone>; MAX_ZONES]>,
    num_pages: usize,
}

impl PhysAllocator {
    pub fn new(map: MemoryMap) -> Self {
        let mut zones = ArrayVec::new();
        let mut num_pages = 0;

        for rg in map {
            let pages_in_rg = rg.size / super::PAGE_SIZE;
            let usable_pages = usable_pages(pages_in_rg);
            if usable_pages <= 1 {
                continue;
            }

            let (reserved, usable) = rg.split_at((pages_in_rg - usable_pages) * super::PAGE_SIZE);
            let zone = Zone::new(
                usable.addr.into(),
                x86_64::align_down(usable.size, super::PAGE_SIZE),
                Block::new_blocks_for_region(reserved, usable_pages),
            );

            num_pages += zone.num_pages;
            zones.push(SpinLock::new(zone));

            assert_eq!(usable.addr.as_usize() & (super::PAGE_SIZE - 1), 0); // Make sure it's aligned
        }

        Self { zones, num_pages }
    }

    pub fn alloc(&self, order: u8) -> PhysFrameRange {
        debug_assert!(order <= MAX_ORDER as u8);

        for zone in &self.zones[1..] {
            let mut zone = zone.lock();
            if let Some(range) = zone.alloc(order) {
                return range;
            }
        }

        panic!(
            "physical memory allocator: out of memory (failed to fulfill order {} alloc)",
            order
        );
    }

    pub fn free(&self, range: PhysFrameRange) {
        for zone in &self.zones {
            let mut zone = zone.lock();
            if zone.pages.contains_range(range) {
                zone.free(range);
                return;
            }
        }

        panic!(
            "attempt to free memory that isn't managed by the PMM ({:?})",
            range
        );
    }

    #[allow(unused)]
    pub fn num_pages(&self) -> usize {
        self.num_pages
    }
}

// Each page of memory has a constant memory overhead of size_of::<PageInfo>(),
// as well as the whole region having a memory overhead of
// blocks_in_region() * size_of::<Block>().
// Let N = number of (PMM) usable memory pages
//     T = total number of pages, usable and unusable
//     W = overhead per page in bytes
// We have the equation
//       total wasted bytes <= 4096 * (T - N)
// N * W + blocks_in_region <= 4096T - 4096N
//           N * (W + 4096) <= 4096T - blocks_in_region
//                    N - 1 < (4096T - blocks_in_region) / (W + 4096)
// Hence: Max usable N = 4096T / (W + 4096) - 1
// Subtract one extra page, just to be safe about padding and alignment
// TODO: should really be blocks_in_region(usable_pages), but this hugely
// complicates the math
fn usable_pages(total_pages: usize) -> usize {
    (4096 * total_pages as usize - blocks_in_region(total_pages))
        / (mem::size_of::<PageInfo>() + 4096)
        - 2
}

fn blocks_in_region(pages: usize) -> usize {
    let max_order_blocks = x86_64::align_up(pages, MAX_ORDER_PAGES) / MAX_ORDER_PAGES;
    // Evaluate the geometric series
    // a = max_order_blocks
    // r = 2
    // n = max_order + 1
    max_order_blocks * (2usize.pow(MAX_ORDER as u32 + 1) - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    test_case!(block_repr, {
        assert_eq!(mem::size_of::<Block>(), 1);
        assert_eq!(mem::align_of::<Block>(), 1);

        // Check that 0 corresponds to Block::Used
        let b: u8 = 0;
        let block = &b as *const u8 as *const Block;
        assert_eq!(unsafe { *block }, Block::Used);
    });
}
