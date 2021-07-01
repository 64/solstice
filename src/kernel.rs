use crate::{
    cpu,
    drivers,
    mm::{map::MemoryMap, pmm::PhysAllocator},
};
use acpi::InterruptModel;
use bootloader::bootinfo::BootInfo;
pub fn kernel_main(info: &BootInfo) {
    drivers::serial::init();
    drivers::vga::text_mode::init().unwrap();
    #[rustfmt::skip]
    {
        println!("  _____       _     _   _             Developed by:");
        println!(" / ____|     | |   | | (_)              - Vinc");
        println!("| (___   ___ | |___| |_ _  ___ ___      - Crally");
        println!(" \\___ \\ / _ \\| / __| __| |/ __/ _ \\     - Mehodin");
        println!(" ____) | (_) | \\__ \\ |_| | (_|  __/     - Alex8675");
        println!("|_____/ \\___/|_|___/\\__|_|\\___\\___|");
        println!();
    };

    cpu::gdt::load();
    cpu::idt::load();

    let map = MemoryMap::new(&info.memory_map);

    PhysAllocator::init(map);

    let acpi = drivers::acpi::init();
    match acpi.interrupt_model {
        InterruptModel::Unknown { .. } => panic!("unsupported acpi interrupt model"),
        InterruptModel::Apic { .. } => {
            if !drivers::acpi::apic_supported() {
                error!("apic: xapic is not supported");
            } else {
                info!("apic: detected xapic support");
            }
        }
        _ => {panic!("unknown acpi interrupt model")}
    };
}
