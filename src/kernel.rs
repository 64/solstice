#![rustfmt::skip]
use crate::cpu;
use crate::drivers;

pub fn kernel_main() {
    drivers::vga::text_mode::init().unwrap();

    println!("  _____       _     _   _             Join us at discord.gg/vnyVmAE");
    println!(" / ____|     | |   | | (_)            Developed by members:");
    println!("| (___   ___ | |___| |_ _  ___ ___    {:11} {:11} {:11}", "vinc", "TBA", "TBA");
    println!(" \\___ \\ / _ \\| / __| __| |/ __/ _ \\   {:11} {:11} {:11}", "Crally", "TBA", "TBA");
    println!(" ____) | (_) | \\__ \\ |_| | (_|  __/   {:11} {:11} {:11}", "Mehodin", "TBA", "TBA");
    println!("|_____/ \\___/|_|___/\\__|_|\\___\\___|   {:11} {:11} {:11}", "Alex8675", "TBA", "TBA");
    println!();

    cpu::gdt::load();
    cpu::idt::load();
}
