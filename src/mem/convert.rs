use x86_64::{PhysAddr, VirtAddr};

pub fn to_phys(virt: VirtAddr) -> PhysAddr {
    PhysAddr::new(virt.as_u64().checked_sub(super::PHYS_OFFSET as u64).unwrap())
}

pub fn to_virt(phys: PhysAddr) -> VirtAddr {
    VirtAddr::new(phys.as_u64().checked_add(super::PHYS_OFFSET as u64).unwrap())
}