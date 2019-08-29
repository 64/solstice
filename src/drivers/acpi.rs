use ::acpi::{AcpiHandler, AmlTable, PhysicalMapping, Fadt, Processor};
use aml::{AmlContext, AmlError, AmlValue};
use core::ptr::NonNull;
use x86_64::{PhysAddr, VirtAddr, instructions::port::{PortRead,PortWrite}};
use aml::value::AmlValue::Package;
use aml::value::AmlType;
use lazy_static::__Deref;
use intrusive_collections::IntrusivePointer;
use core::borrow::{BorrowMut, Borrow};
use alloc::vec::Vec;
use core::marker::PhantomData;

static mut SLP_TYPA:u64 = 0;
pub fn init() -> ::acpi::Acpi {
    let mut our_acpi = unsafe {acpi::search_for_rsdp_bios(&mut Acpi).expect("ACPI table parsing failed")};

    debug!("acpi: found tables");
    let mut ctx = AmlContext::new();
    match unsafe {core::ptr::read(&our_acpi.dsdt)} {
        Some(dsdt) => {
            parse_table(&mut ctx, &dsdt);
            debug!("acpi: parsed dsdt");
        }
        None => {
            warn!("acpi: DSDT not found!");
        }
    }
    for (i, ssdt) in our_acpi.ssdts.iter().enumerate() {
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
                    debug!("acpi: getting SLP_TYPA");
                    unsafe {
                        SLP_TYPA = v[0].as_integer().unwrap() << 10;
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
    return our_acpi;
}
pub fn enable(acpi: &mut ::acpi::Acpi) {

    let fadt = unsafe { acpi.fadt.unwrap().as_ptr() };
    let mut fadt = unsafe { fadt.clone().read() };
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
        } else {
            debug!("ACPI cannot be enabled");
        }
    } else {
        warn!("ACPI already enabled");
    }

}
pub fn shutdown(acpi: &mut ::acpi::Acpi) {
    let fadt = unsafe { acpi.fadt.unwrap().as_ptr() };
    let mut fadt = unsafe { fadt.clone().read() };
    loop {
        unsafe { PortWrite::write_to_port(fadt.pm1a_control_block as u16, (SLP_TYPA | (1 << 13)) as u16); }
        if (fadt.pm1b_control_block != 0) {
            unsafe { PortWrite::write_to_port(fadt.pm1b_control_block as u16, (SLP_TYPA | (1 << 13)) as u16); }
        }
        //wait till dead
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
