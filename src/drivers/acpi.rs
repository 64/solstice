use acpi::{AcpiHandler, AmlTable, PhysicalMapping, Fadt};
use aml::{AmlContext, AmlError, AmlValue};
use core::ptr::NonNull;
use x86_64::{PhysAddr, VirtAddr, instructions::port::{PortRead,PortWrite}};
use aml::value::AmlValue::Package;
use aml::value::AmlType;
use lazy_static::__Deref;
use intrusive_collections::IntrusivePointer;
use core::borrow::BorrowMut;

static mut SLP_TYPa:u64 = 0;
static mut acpip: *mut acpi::Acpi = 0 as *mut acpi::Acpi;
pub fn init() {
    let mut acpi = unsafe { acpi::search_for_rsdp_bios(&mut Acpi).expect("ACPI table parsing failed") };

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
    match v {
        AmlValue::Name(p) => {
            match p.deref() {
                AmlValue::Package(v) => {
                    debug!("acpi: getting SLP_TYPa");
                    unsafe {
                        SLP_TYPa = v[0].as_integer().unwrap();
                    }
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
    unsafe {
        acpip = acpi.borrow_mut();
    }
    debug!("acpi: done");
}
pub fn enable() {
    let fadt = unsafe { (*acpip).fadt.unwrap() };
    let fadt = unsafe { fadt.as_ref() };
    let mut readval:u16 = 0;
    // SCI_EN is 1
    readval = unsafe { PortRead::read_from_port(fadt.pm1a_control_block as u16) };
    if (readval & 1 == 0) {
        if (fadt.smi_cmd_port != 0 && fadt.acpi_enable != 0) {
            unsafe { PortWrite::write_to_port(fadt.smi_cmd_port as u16, fadt.acpi_enable); }
            readval = unsafe { PortRead::read_from_port(fadt.pm1a_control_block as u16) };
            while (readval & 1 == 0) {
                readval = unsafe { PortRead::read_from_port(fadt.pm1a_control_block as u16) };
            }

            if (fadt.pm1b_control_block != 0) {
                readval = unsafe { PortRead::read_from_port(fadt.pm1b_control_block as u16) };
                while (readval & 1 == 0) {
                    readval = unsafe { PortRead::read_from_port(fadt.pm1b_control_block as u16) };
                }
            }
            return;
        }
    }
}
pub fn shutdown() {
    let fadt = unsafe { (*acpip).fadt.unwrap() };
    let fadt = unsafe { fadt.as_ref() };
    unsafe { PortWrite::write_to_port(fadt.pm1a_control_block as u16, (SLP_TYPa | 1 << 13) as u16); }
    if (fadt.pm1b_control_block != 0) {
        unsafe { PortWrite::write_to_port(fadt.pm1b_control_block as u16, (SLP_TYPa | 1 << 13) as u16); }
    }
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
