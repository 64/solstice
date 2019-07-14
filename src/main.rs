#![no_std]
#![no_main]
#![test_runner(crate::testing::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![feature(custom_inner_attributes)]
#![feature(core_intrinsics)]
#![feature(asm)]
#![feature(alloc_layout_extra)]

#[macro_use]
extern crate log;

#[macro_use]
extern crate lazy_static;

#[macro_use]
mod macros;

mod cpu;
mod drivers;
mod ds;
mod kernel;
mod mem;
mod testing;

use bootloader::BootInfo;

#[no_mangle]
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    kernel::kernel_main(boot_info);

    // Run tests
    #[cfg(test)]
    test_main();

    info!("nothing to do, halting...");

    halt_loop();
}

#[allow(unused_imports)]
use core::panic::PanicInfo;

#[panic_handler]
#[cfg(not(test))]
#[allow(clippy::empty_loop)]
fn panic(info: &PanicInfo) -> ! {
    error!("{}", info);
    halt_loop();
}

fn halt_loop() -> ! {
    loop {
        x86_64::instructions::interrupts::disable();
        x86_64::instructions::hlt();
    }
}
