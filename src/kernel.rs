use crate::{cpu, drivers, mem::bump::BumpAllocator};
use bootloader::bootinfo::MemoryMap;

pub fn kernel_main(mem_map: &MemoryMap) {
    drivers::serial::init();
    drivers::vga::text_mode::init().unwrap();

    #[rustfmt::skip]
    {
        println!("  _____       _     _   _             Join us at discord.gg/vnyVmAE");
        println!(" / ____|     | |   | | (_)            Developed by members:");
        println!("| (___   ___ | |___| |_ _  ___ ___    {:11} {:11} {:11}", "vinc", "TBA", "TBA");
        println!(" \\___ \\ / _ \\| / __| __| |/ __/ _ \\   {:11} {:11} {:11}", "Crally", "TBA", "TBA");
        println!(" ____) | (_) | \\__ \\ |_| | (_|  __/   {:11} {:11} {:11}", "Mehodin", "TBA", "TBA");
        println!("|_____/ \\___/|_|___/\\__|_|\\___\\___|   {:11} {:11} {:11}", "Alex8675", "TBA", "TBA");
        println!();
    };

    cpu::gdt::load();
    cpu::idt::load();

    for entry in mem_map.iter() {
        debug!("{:?}, {:?}", entry.range, entry.region_type);
    }

    let _bump = BumpAllocator::new(&mem_map);
}
