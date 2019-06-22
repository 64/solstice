#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::testing::test_runner)]
#![reexport_test_harness_main = "test_main"]
#[macro_use]
extern crate log;
#[macro_use]
extern crate ddos_drivers as drivers;
extern crate ddos_ds as ds;

mod cpu;
mod kernel;
mod qemu;
mod testing;

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    kernel::kernel_main();

    // Run tests
    #[cfg(test)]
    test_main();

    info!("nothing to do, halting...");

    loop {
        // x86_64::instructions::interrupts::enable();
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
#[cfg(not(test))]
#[allow(clippy::empty_loop)]
fn panic(info: &PanicInfo) -> ! {
    error!("{}", info);

    // Halt CPU
    loop {
        x86_64::instructions::interrupts::disable();
        x86_64::instructions::hlt();
    }
}
