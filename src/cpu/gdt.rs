use lazy_static::lazy_static;
use x86_64::structures::gdt::{Descriptor, DescriptorFlags, GlobalDescriptorTable};

lazy_static! {
    static ref GDT: GlobalDescriptorTable = {
        let mut gdt = GlobalDescriptorTable::new();

        // Null segment
        gdt.add_entry(Descriptor::UserSegment(0));

        // Kernel code segment
        gdt.add_entry(Descriptor::kernel_code_segment());

        // Kernel data segment
        let flags = DescriptorFlags::USER_SEGMENT | DescriptorFlags::PRESENT;
        gdt.add_entry(Descriptor::UserSegment(flags.bits()));

        gdt
    };
}

pub fn load() {
    GDT.load();
}
