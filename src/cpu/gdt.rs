use lazy_static::lazy_static;
use x86_64::structures::gdt::{Descriptor, DescriptorFlags, GlobalDescriptorTable};

lazy_static! {
    static ref GDT: GlobalDescriptorTable = {
        let mut gdt = GlobalDescriptorTable::new();

        // Kernel code segment
        gdt.add_entry(Descriptor::kernel_code_segment());

        // Kernel data segment
        let flags = DescriptorFlags::USER_SEGMENT | DescriptorFlags::PRESENT;
        gdt.add_entry(Descriptor::UserSegment(flags.bits() | (1 << 41)));

        gdt
    };
}

pub fn load() {
    GDT.load();

    unsafe {
        use x86_64::{
            instructions::segmentation as seg,
            structures::gdt::SegmentSelector,
            PrivilegeLevel,
        };

        let code_segment = SegmentSelector::new(1, PrivilegeLevel::Ring0);
        let data_segment = SegmentSelector::new(2, PrivilegeLevel::Ring0);

        seg::load_ds(data_segment);
        seg::load_es(data_segment);
        seg::load_fs(data_segment);
        seg::load_gs(data_segment);
        seg::load_ss(data_segment);
        seg::set_cs(code_segment);
    }

    info!("GDT loaded");
}
