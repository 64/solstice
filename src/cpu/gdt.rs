use lazy_static::lazy_static;
use x86_64::{
    instructions::tables::load_tss,
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable},
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

            stack_start + STACK_SIZE
        };

        tss
    };
}

lazy_static! {
    static ref GDT: GlobalDescriptorTable = {
        let mut gdt = GlobalDescriptorTable::new();

        // Kernel code segment
        gdt.add_entry(Descriptor::kernel_code_segment());

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

        let null_segment = SegmentSelector::new(0, PrivilegeLevel::Ring0);
        let code_segment = SegmentSelector::new(1, PrivilegeLevel::Ring0);
        let tss_segment = SegmentSelector::new(2, PrivilegeLevel::Ring0);

        seg::load_ds(null_segment);
        seg::load_es(null_segment);
        seg::load_fs(null_segment);
        seg::load_gs(null_segment);
        seg::load_ss(null_segment);
        seg::set_cs(code_segment);
        load_tss(tss_segment);
    }

    debug!("gdt: loaded");
}
