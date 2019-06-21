use core::fmt;
use ddos_ds::SpinLock;
use lazy_static::lazy_static;

const WIDTH: usize = 80;
// const HEIGHT: usize = 25;

pub struct Writer {
    buf: &'static mut [u16],
    x: usize,
    y: usize,
}

impl Writer {
    pub fn write_byte(&mut self, ch: u8) {
        self.buf[self.y * WIDTH + self.x] = (0x0B << 8) | u16::from(ch);

        self.x += 1; // TODO: Properly handle newlines, tabs, scrolling etc
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            // TODO: Handle non-ascii chars
            self.write_byte(byte);
        }
    }
}

// Everything below here is just glue for the println! and print! macros

lazy_static! {
    pub static ref WRITER: SpinLock<Writer> = SpinLock::new(Writer {
        buf: unsafe { core::slice::from_raw_parts_mut(0xB8000 as *mut u16, 80 * 25) },
        x: 0,
        y: 0,
    });
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_text::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}
