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
mod macros;

mod cpu;
mod drivers;
mod ds;
mod testing;
mod kernel;
mod mem;
mod qemu;

use bootloader::BootInfo;
#[allow(unused_imports)]
use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    kernel::kernel_main();

    info!(
        "Physical memory offset: {:#x}",
        boot_info.physical_memory_offset
    );

    let rip: u64;
    unsafe { asm!("lea (%rip), $0" : "=r"(rip) ::: "volatile") };
    info!("Executing at {:#x}", rip);

    let rsp: u64;
    unsafe { asm!("mov %rsp, $0" : "=r"(rsp) ::: "volatile") };
    info!("Stack at {:#x}", rsp);

    for entry in boot_info.memory_map.iter() {
        debug!("{:?}, {:?}", entry.range, entry.region_type);
    }

    // Run tests
    #[cfg(test)]
    test_main();

    info!("nothing to do, halting...");

    abort();
}

#[panic_handler]
#[cfg(not(test))]
#[allow(clippy::empty_loop)]
fn panic(info: &PanicInfo) -> ! {
    error!("{}", info);
    abort();
}

fn abort() -> ! {
    loop {
        x86_64::instructions::interrupts::disable();
        x86_64::instructions::hlt();
    }
}
