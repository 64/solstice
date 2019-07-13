use crate::mem::bump::BumpAllocator;
use core::mem;
use arrayvec::ArrayVec;
use x86_64::VirtAddr;

pub const MAX_ZONES: usize = 64;

struct Zone {
    addr: VirtAddr,
    size: usize,
}

#[repr(u8)]
enum Block {
    None
}

struct PageInfo;

pub struct PhysAllocator {
    zones: ArrayVec<[Zone; MAX_ZONES]>,
}

impl PhysAllocator {
    pub fn new(bump: BumpAllocator) -> Self {
        let mut zones = ArrayVec::new();

        for region in &bump {
            zones.push(Zone {
                addr: VirtAddr::new(region.addr.as_u64() + super::PHYS_OFFSET),
                size: region.size as usize,
            });
        }

        // Don't allocate after this point

        Self { zones }
    }
}

// Each page of memory has a constant memory overhead of 2 * size_of::<Block>() + size_of::<PageInfo>()
// Let N = number of (PMM) usable memory
//     T = total number of pages, usable and unusable
//     W = overhead per page in bytes
// We have the equation
// total wasted bytes <= 4096 * (T - N)
//              N * W <= 4096T - 4096N
//     N * (W + 4096) <= 4096T
//              N - 1 <= 4096T / (W + 4096)         (due to integer division truncation)
// Hence: Usable N = 4096T / (W + 4096)
fn usable_pages(total_pages: usize) -> usize {
    4096 * total_pages / (2 * mem::size_of::<Block>() + mem::size_of::<PageInfo>() + 4096) - 1
}