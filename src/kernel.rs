use crate::{
    cpu,
    drivers,
    mem::{bump::BumpAllocator, pmm::PhysAllocator},
};
use bootloader::bootinfo::MemoryMap;

pub fn kernel_main(mem_map: &MemoryMap) {
    drivers::serial::init();
    drivers::vga::text_mode::init().unwrap();

    #[rustfmt::skip]
    {
        println!("  _____       _     _   _             Join us at discord.gg/vnyVmAE");
        println!(" / ____|     | |   | | (_)            Developed by members:");
        println!("| (___   ___ | |___| |_ _  ___ ___      - Vinc");
        println!(" \\___ \\ / _ \\| / __| __| |/ __/ _ \\     - Crally");
        println!(" ____) | (_) | \\__ \\ |_| | (_|  __/     - Mehodin");
        println!("|_____/ \\___/|_|___/\\__|_|\\___\\___|     - Alex8675");
        println!();
    };

    cpu::gdt::load();
    cpu::idt::load();

    let bump = BumpAllocator::new(&mem_map);

    let _pmm = PhysAllocator::new(bump);
}
