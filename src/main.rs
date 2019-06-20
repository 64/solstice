#![no_std]
#![no_main]

use core::panic::PanicInfo;

static TEST: &[u8] = b"HELLO WORLD";

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let vga_buffer = unsafe { core::slice::from_raw_parts_mut(0xB8000 as *mut u16, 80 * 25) };

    for (i, &byte) in TEST.iter().enumerate() {
        vga_buffer[i] = (0xB << 8) | (byte as u16);
    }

    loop {}
}
