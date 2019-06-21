#![no_std]
#![no_main]

#[macro_use]
extern crate log;

#[macro_use]
extern crate ddos_drivers as drivers;

extern crate ddos_ds as ds;

mod cpu;

use core::panic::PanicInfo;
#[panic_handler]
#[allow(clippy::empty_loop)]
fn panic(info: &PanicInfo) -> ! {
    error!("{}", info);
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    drivers::vga_text::init().unwrap();

    cpu::gdt::load();

    debug!("test 1");
    info!("test 2");
    warn!("test 3");
    error!("test 4");

    let x = "test 5";
    dbg!(x);

    loop {
        // x86_64::instructions::interrupts::enable();
        x86_64::instructions::hlt();
    }
}
