use x86_64::{PhysAddr, VirtAddr};

pub fn to_phys(virt: VirtAddr) -> PhysAddr {
    PhysAddr::new(
        virt.as_usize()
            .checked_sub(super::PHYS_OFFSET)
            .unwrap(),
    )
}

pub fn to_virt(phys: PhysAddr) -> VirtAddr {
    VirtAddr::new(
        phys.as_usize()
            .checked_add(super::PHYS_OFFSET)
            .unwrap(),
    )
}
