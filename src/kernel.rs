use crate::cpu;

pub fn kernel_main() {
    drivers::vga_text::init().unwrap();

    cpu::gdt::load();
}
