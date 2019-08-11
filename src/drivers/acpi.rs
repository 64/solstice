use crate::mm::{self, addr_space::AddrSpace};
use acpi::{AcpiHandler, PhysicalMapping};
use core::ptr::NonNull;
use x86_64::{
    structures::paging::{PageTableFlags, PhysFrame},
    PhysAddr,
    VirtAddr,
};

pub fn init() {
    unsafe {
        let acpi = acpi::search_for_rsdp_bios(&mut Acpi).expect("ACPI table parsing failed");
        dbg!(acpi);
    }

    debug!("acpi: initialised");
}

struct Acpi;

impl AcpiHandler for Acpi {
    fn map_physical_region<T>(
        &mut self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<T> {
        let start_virt = VirtAddr::from(PhysAddr::new(physical_address));

        PhysicalMapping {
            physical_start: physical_address,
            virtual_start: NonNull::new(start_virt.as_mut_ptr()).expect("acpi mapped null ptr"),
            region_length: size,
            mapped_length: size,
        }
    }

    fn unmap_physical_region<T>(&mut self, _region: PhysicalMapping<T>) {}
}
