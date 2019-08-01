use core::{
    fmt::{Result, Write},
    sync::atomic::{AtomicUsize, Ordering},
};

const VGA_BUFFER: *mut u16 = 0xb8000 as *mut _;
const SCREEN_SIZE: usize = 80 * 25;

pub static CURRENT_OFFSET: AtomicUsize = AtomicUsize::new(160);

pub struct Printer;

impl Printer {
    pub fn clear_screen(&mut self) {
        for i in 0..SCREEN_SIZE {
            unsafe {
                VGA_BUFFER.offset(i as isize).write_volatile(0xf00);
            }
        }

        CURRENT_OFFSET.store(0, Ordering::Relaxed);
    }
}

impl Write for Printer {
    fn write_str(&mut self, s: &str) -> Result {
        for byte in s.bytes() {
            let index = CURRENT_OFFSET.fetch_add(1, Ordering::Relaxed) as isize;

            unsafe {
                VGA_BUFFER.offset(index).write_volatile(0x4f00 | u16::from(byte));
            }
        }

        Ok(())
    }
}
