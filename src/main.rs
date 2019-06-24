#![no_std]
#![no_main]
#![test_runner(crate::testing::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![feature(custom_inner_attributes)]
#![feature(core_intrinsics)]
#![feature(asm)]

#[macro_use]
extern crate log;

#[macro_use]
mod drivers;

// TODO: Ideally put this above drivers so we can test that too
// Would require moving println et al out of the drivers module
#[macro_use]
mod testing;

mod cpu;
mod ds;
mod kernel;
mod qemu;

#[allow(unused_imports)]
use core::panic::PanicInfo;
use bootloader::BootInfo;

#[no_mangle]
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    kernel::kernel_main();

    info!("Physical memory offset: 0x{:x}", boot_info.physical_memory_offset);

    // Run tests
    #[cfg(test)]
    test_main();

    dbg!("This is a dbg");
    warn!("This is a warning");
    trace!();

    info!("nothing to do, \x1B[31mhalting...");

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
