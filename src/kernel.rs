use crate::{
    cpu,
    drivers,
    mem::{map::MemoryMap, pmm::PhysAllocator},
};
use bootloader::bootinfo::BootInfo;

pub fn kernel_main(info: &BootInfo) {
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

    let map = MemoryMap::new(&info.memory_map);

    let pmm = PhysAllocator::new(map);

    dbg!("foo");
    for o in 0..5 {
        let a = pmm.alloc(o);
        pmm.free(a);
    }
}
