use crate::{
    cpu,
    drivers,
    mm::{map::MemoryMap, pmm::PhysAllocator},
};
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
        println!("|_____/ \\___/|_|___/\\__|_|\\___\\___|   -trash");
        println!();
    };

    cpu::gdt::load();
    cpu::idt::load();

    let map = MemoryMap::new(&info.memory_map);
    PhysAllocator::init(map);

    let mut acpi = drivers::acpi::init();
    debug!("ACPI initialized");
    drivers::acpi::enable(&mut acpi);
    debug!("ACPI enabled");

    debug!("Nothing to do, shutting down...");
    drivers::acpi::shutdown(&mut acpi);
    unreachable!();
}
