use acpi::{Acpi, AcpiHandler, AmlTable, PhysicalMapping};
use aml::{AmlContext, AmlError};
use core::ptr::NonNull;
use x86_64::{PhysAddr, VirtAddr};

pub fn init() -> Acpi {
    let acpi = unsafe {
        acpi::search_for_rsdp_bios(&mut DummyAcpiHandler).expect("ACPI table parsing failed")
    };

    debug!("acpi: found tables");

    let mut ctx = AmlContext::new();

    if let Some(dsdt) = &acpi.dsdt {
        parse_table(&mut ctx, dsdt).expect("AML DSDT parsing failed");
        debug!("acpi: parsed dsdt");
    }

    for (i, ssdt) in acpi.ssdts.iter().enumerate() {
        parse_table(&mut ctx, ssdt).expect("AML SSDT parsing failed");
        debug!("acpi: parsed ssdt {}", i);
    }

    debug!("acpi: done!");

    acpi
}

fn parse_table(ctx: &mut AmlContext, table: &AmlTable) -> Result<(), AmlError> {
    let virt = VirtAddr::from(PhysAddr::new(table.address as u64));

    ctx.parse_table(unsafe { core::slice::from_raw_parts(virt.as_ptr(), table.length as usize) })
}

struct DummyAcpiHandler;

impl AcpiHandler for DummyAcpiHandler {
    fn map_physical_region<T>(
        &mut self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<T> {
        let start_virt = VirtAddr::from(PhysAddr::new(physical_address as u64));

        PhysicalMapping {
            physical_start: physical_address,
            virtual_start: NonNull::new(start_virt.as_mut_ptr()).expect("acpi mapped null ptr"),
            region_length: size,
            mapped_length: size,
        }
    }

    fn unmap_physical_region<T>(&mut self, _region: PhysicalMapping<T>) {}
}
