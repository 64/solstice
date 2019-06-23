use crate::cpu;

pub fn kernel_main() {
    drivers::vga_text::init().unwrap();

    println!(" __          ___           _  ____   _____  Join us at discord.gg/vnyVmAE");
    println!(" \\ \\        / (_)         | |/ __ \\ / ____| Developed by members:");
    println!("  \\ \\  /\\  / / _ _ __   __| | |  | | (___   {:11} {:11} {:11}", "vinc", "Alex8675", "TBA");
    println!("   \\ \\/  \\/ / | | '_ \\ / _` | |  | |\\___ \\  {:11} {:11} {:11}", "Crally", "TBA", "TBA");
    println!("    \\  /\\  /  | | | | | (_| | |__| |____) | {:11} {:11} {:11}", "Mehodin", "TBA", "TBA");
    println!("     \\/  \\/   |_|_| |_|\\__,_|\\____/|_____/  {:11} {:11} {:11}", "Styxs", "TBA", "TBA");
    println!("");

    cpu::gdt::load();
    cpu::idt::load();
}
