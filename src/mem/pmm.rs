use crate::mem::bump::BumpAllocator;
use arrayvec::ArrayVec;
use x86_64::VirtAddr;

pub const MAX_ZONES: usize = 64;

struct Zone {
    addr: VirtAddr,
    size: usize,
}

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
