use crate::cpu;

pub fn kernel_main() {
    drivers::vga_text::init().unwrap();
    
    println!("      _     _  ____   _____    Join us at https://discord.gg/vnyVmAE");
    println!("     | |   | |/ __ \\ / ____|   Developed by members:");
    println!("   __| | __| | |  | | (___     {:12} {:12} {:12}", "vinc", "Alex8675", "TBA");
    println!("  / _` |/ _` | |  | |\\___ \\    {:12} {:12} {:12}", "Crally", "TBA", "TBA");
    println!(" | (_| | (_| | |__| |____) |   {:12} {:12} {:12}", "Mehodin", "TBA", "TBA");
    println!("  \\__,_|\\__,_|\\____/|_____/    {:12} {:12} {:12}", "Styxs", "TBA", "TBA");
    println!("");

    cpu::gdt::load();
    cpu::idt::load();
}
