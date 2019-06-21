#![no_std]
#![no_main]

extern crate ddos_ds as ds;

use core::panic::PanicInfo;

static TEST: &[u8] = b"HELLO WORLD";

#[panic_handler]
#[allow(clippy::empty_loop)]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let vga_buffer = unsafe { core::slice::from_raw_parts_mut(0xB8000 as *mut u16, 80 * 25) };

    for (i, &byte) in TEST.iter().enumerate() {
        vga_buffer[i] = (0x0B << 8) | (u16::from(byte));
    }

    let test_lock = ds::SpinLock::new(5);

    {
        *test_lock.lock() = 10;
    }

    #[allow(clippy::empty_loop)]
    loop {}
}