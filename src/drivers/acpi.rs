use acpi::{AcpiHandler, AmlTable, PhysicalMapping};
use aml::{AmlContext, AmlError, AmlValue};
use core::ptr::NonNull;
use x86_64::{PhysAddr, VirtAddr};
use aml::value::AmlValue::Package;
use aml::value::AmlType;
use lazy_static::__Deref;
pub fn init() {
    let acpi = unsafe { acpi::search_for_rsdp_bios(&mut Acpi).expect("ACPI table parsing failed") };

    debug!("acpi: found tables {:#?}", acpi);
    let mut ctx = AmlContext::new();

    if let Some(dsdt) = &acpi.dsdt {
        parse_table(&mut ctx, dsdt).expect("AML DSDT parsing failed");
        debug!("acpi: parsed dsdt");
    }

    for (i, ssdt) in acpi.ssdts.iter().enumerate() {
        parse_table(&mut ctx, ssdt).expect("AML SSDT parsing failed");
        debug!("acpi: parsed ssdt {}", i);
    }


    let mut name = aml::AmlName::from_str("_S5_").expect("Could not get AmlName");
    let root = aml::AmlName::root();
    let mut name = ctx.namespace.search(&name, &root).expect("Could not get actual name");
    let v = ctx.namespace.get(name).expect("Could not get AmlValue");
    let mut SLP_TYPa:u64 = 52;
    match v {
        AmlValue::Name(p) => {
            match p.deref() {
                AmlValue::Package(v) => {
                    debug!("acpi: getting SLP_TYPa");
                    SLP_TYPa = v[0].as_integer().unwrap();
                }
                _ => {
                    unreachable!();
                }
            }
        },
        _ => {
            unreachable!();
        }
    }

    debug!("acpi: done!");
}

fn parse_table(ctx: &mut AmlContext, table: &AmlTable) -> Result<(), AmlError> {
    let virt = VirtAddr::from(PhysAddr::new(table.address));

    ctx.parse_table(unsafe { core::slice::from_raw_parts(virt.as_ptr(), table.length as usize) })
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
