use lazy_static::lazy_static;
use x86_64::{
    instructions::tables::load_tss,
    structures::{
        gdt::{Descriptor, DescriptorFlags, GlobalDescriptorTable},
        tss::TaskStateSegment,
    },
    VirtAddr,
};

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };

        tss
    };
}

lazy_static! {
    static ref GDT: GlobalDescriptorTable = {
        let mut gdt = GlobalDescriptorTable::new();

        // Kernel code segment
        gdt.add_entry(Descriptor::kernel_code_segment());

        // Kernel data segment
        let flags = DescriptorFlags::USER_SEGMENT | DescriptorFlags::PRESENT;
        gdt.add_entry(Descriptor::UserSegment(flags.bits() | (1 << 41)));

        // TSS segment
        gdt.add_entry(Descriptor::tss_segment(&TSS));

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
        let tss_segment = SegmentSelector::new(3, PrivilegeLevel::Ring0);

        seg::load_ds(data_segment);
        seg::load_es(data_segment);
        seg::load_fs(data_segment);
        seg::load_gs(data_segment);
        seg::load_ss(data_segment);
        seg::set_cs(code_segment);
        load_tss(tss_segment);
    }

    info!("GDT loaded");
}
