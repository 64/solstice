use ::acpi::{AcpiHandler, AmlTable, PhysicalMapping};
use aml::{AmlContext, AmlError, AmlValue};
use core::ptr::NonNull;
use x86_64::{PhysAddr, VirtAddr, instructions::port::{PortRead,PortWrite}};
use lazy_static::__Deref;
use acpi::{AcpiTables, AcpiError, InterruptModel};
use acpi::sdt::Signature;
use acpi::platform::address::GenericAddress;
use acpi::platform::ProcessorInfo;

static mut SLP_TYPA:u64 = 0;
pub fn init() -> AcpiObject {
    let our_acpi = unsafe {acpi::AcpiTables::search_for_rsdp_bios(Acpi)}.expect("ACPI table parsing failed");

    debug!("acpi: found tables");
    let mut ctx = AmlContext::new();
    match unsafe {core::ptr::read(&our_acpi.dsdt)} {
        Some(dsdt) => {
            parse_table(&mut ctx, &dsdt).expect("AML DSDT parsing failed");
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


    let name = aml::AmlName::from_str("_S5_").expect("Could not get AmlName");
    let root = aml::AmlName::root();
    let name = ctx.namespace.search(&name, &root).expect("Could not get actual name");
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
    return AcpiObject::new(our_acpi);
}
pub fn enable(acpi: &mut AcpiObject) {
    let pm1a_control_block = unsafe {*(acpi.fadt.pm1a_control_block()
        .expect("Could not get pm1a_control_block of ACPI!")
        .address as *const u16)};
    let mut readval:u16 = unsafe { PortRead::read_from_port(pm1a_control_block) };
    if readval & 1 == 0 {
        if acpi.fadt.smi_cmd_port != 0 && acpi.fadt.acpi_enable != 0 {
            unsafe { PortWrite::write_to_port(acpi.fadt.smi_cmd_port as u16, acpi.fadt.acpi_enable); }
            readval = unsafe { PortRead::read_from_port(pm1a_control_block) };
            while readval & 1 == 0 {
                readval = unsafe { PortRead::read_from_port(pm1a_control_block) };
            }
            match acpi.fadt.pm1b_control_block().expect("Could not get pm1b_control_block") {
                None => {}
                Some(pm1b_control_block_addr) => {
                    let pm1b_control_block = unsafe {*(pm1b_control_block_addr.address as *const u16)};
                    readval = unsafe { PortRead::read_from_port(pm1b_control_block as u16) };
                    while readval & 1 == 0 {
                        readval = unsafe { PortRead::read_from_port(pm1b_control_block as u16) };
                    }
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
pub fn shutdown(acpi: &mut AcpiObject) {
    let pm1a_control_block = unsafe {*(acpi.fadt.pm1a_control_block()
        .expect("Could not get pm1a_control_block of ACPI!")
        .address as *const u16)};
    loop {
        unsafe { PortWrite::write_to_port(pm1a_control_block as u16, (SLP_TYPA | (1 << 13)) as u16); }
        match acpi.fadt.pm1b_control_block().expect("Could not get pm1b_control_block") {
            None => {}
            Some(pm1b_control_block_addr) => {
                let pm1b_control_block = unsafe {*(pm1b_control_block_addr.address as *const u16)};
                unsafe { PortWrite::write_to_port(pm1b_control_block as u16, (SLP_TYPA | (1 << 13)) as u16); }
            }
        }
        //wait till dead
    }
}
fn parse_table(ctx: &mut AmlContext, table: &AmlTable) -> Result<(), AmlError> {
    let virt = VirtAddr::new(table.address as u64);
    ctx.parse_table(unsafe { core::slice::from_raw_parts(virt.as_ptr(), table.length as usize) })
}

pub struct Acpi;
impl Clone for Acpi {
    fn clone(&self) -> Self {
        Acpi
    }
}
impl AcpiHandler for Acpi {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<Self, T> {
        let start_virt = VirtAddr::new(physical_address as u64);
        PhysicalMapping::new(physical_address, NonNull::new(start_virt.as_mut_ptr()).expect("acpi mapped null ptr"), size, size, Acpi)

    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {}
}
pub struct AcpiObject {
    pub tables: AcpiTables<Acpi>,
    pub fadt: PhysicalMapping<Acpi, acpi::fadt::Fadt>,
    pub madt: PhysicalMapping<Acpi, acpi::madt::Madt>,
    pub interrupt_model: InterruptModel,
    pub processor_info: Option<ProcessorInfo>
}
impl AcpiObject {
    pub fn new(acpi_tables: AcpiTables<Acpi>) -> Self {
        let madt = unsafe { acpi_tables.get_sdt::<acpi::madt::Madt>(Signature::MADT) }
            .expect("Could not get MADT")
            .expect("Could not get physical mapping");
        let fadt = unsafe { acpi_tables.get_sdt::<acpi::fadt::Fadt>(Signature::FADT) }
            .expect("Could not get FADT")
            .expect("Could not get physical mapping");
        let madt_result = madt.parse_interrupt_model()
            .expect("Could not get interrupt model");
        AcpiObject {
            tables: acpi_tables,
            fadt,
            madt,
            interrupt_model: madt_result.0,
            processor_info: madt_result.1
        }
    }
}
pub fn apic_supported() -> bool {
    let cpuid = unsafe {core::arch::x86_64::__cpuid(0x1)};
    (cpuid.edx & (1 << 9)) != 0
}