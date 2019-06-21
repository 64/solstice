use core::fmt;
use ddos_ds::SpinLock;
use lazy_static::lazy_static;
use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};

const WIDTH: usize = 80;
const HEIGHT: usize = 25;

pub struct Writer {
    buf: &'static mut [u16],
    x: usize,
    y: usize,
}

impl Writer {
    /// Handles escape characters for writing to the screen.
    ///
    /// Done: \n, \t
    /// TODO: All the other escape characters (\r, etc...)
    ///
    /// Returns true if the character is an escape character, false if it's not.
    /// If it's an escape character, then the `x` and `y` positions do not
    /// have to be incremented because that's handled inside this function.
    pub fn handle_escapes(&mut self, ch: u8) -> bool {
        match ch {
            b'\n' => {
                self.y += 1;
                self.x = 0;
                true
            }
            b'\t' => {
                self.write_string("    ");
                true
            }
            _ => false,
        }
    }

    /// Handles scrolling
    /// In essence this just pushes everything up by decrementing their `y`
    /// position by 1 This overwrites row 0
    pub fn handle_scrolling(&mut self) {
        if self.y < HEIGHT {
            return;
        }

        unsafe {
            core::intrinsics::volatile_copy_memory(
                self.buf.as_mut_ptr(),
                self.buf[WIDTH..].as_mut_ptr(),
                WIDTH * (HEIGHT - 1),
            );
        }

        self.y = HEIGHT - 1;
        self.x = 0;
    }

    /// Handles position of the cursor, does not handle scrolling.
    pub fn update_cursor_position(&mut self) {
        self.x += 1;

        if self.x >= WIDTH {
            self.x = 0;
            self.y += 1;
        }
    }

    /// Write a byte to the screen, handles escape characters and updates
    /// positions.
    pub fn write_byte(&mut self, ch: u8) {
        if self.handle_escapes(ch) {
            return;
        }

        self.handle_scrolling();

        // Actually write the character to the screen, escapes have been handled
        // previously no need to worry about those anymore.
        self.buf[self.y * WIDTH + self.x] = (0x0B << 8) | u16::from(ch);

        self.update_cursor_position();
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            // TODO: Handle non-ascii chars
            self.write_byte(byte);
        }
    }
}

// Everything below here is just glue for the 'println!, print!, info!, debug!,
// error!, warn!' macros

// Need a separate struct so we can implement Log trait
pub struct SpinLockWriter(SpinLock<Writer>);

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&*WRITER).map(|()| {
        #[cfg(debug_assertions)]
        log::set_max_level(LevelFilter::Debug);

        #[cfg(not(debug_assertions))]
        log::set_max_level(LevelFilter::Info);
    })
}

lazy_static! {
    pub static ref WRITER: SpinLockWriter = SpinLockWriter(SpinLock::new(Writer {
        buf: unsafe { core::slice::from_raw_parts_mut(0xB8000 as *mut u16, 80 * 25) },
        x: 0,
        y: 0,
    }));
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

#[macro_export]
// Liftedn from standard library
macro_rules! dbg {
    () => {
        println!("[DEBUG {}:{}]", file!(), line!());
    };
    ($val:expr) => {
        // Use of `match` here is intentional because it affects the lifetimes
        // of temporaries - https://stackoverflow.com/a/48732525/1063961
        match $val {
            tmp => {
                println!(
                    "[DEBUG {}:{}] {} = {:#?}",
                    file!(),
                    line!(),
                    stringify!($val),
                    &tmp
                );
                tmp
            }
        }
    };
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.0.lock().write_fmt(args).unwrap();
}

impl Log for SpinLockWriter {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // TODO: Color
            println!("[{}] {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}
