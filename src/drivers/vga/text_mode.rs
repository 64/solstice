use crate::ds::SpinLock;
use core::fmt;
use lazy_static::lazy_static;
use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};
use volatile::Volatile;
use x86_64::instructions::port::{PortRead, PortWrite};

use crate::drivers::serial;

const TERMINAL_BUFFER: usize = 0xB8000;
const WIDTH: usize = 80;
const HEIGHT: usize = 25;

#[repr(u8)]
pub enum Color {
    Black = 0x00,
    Blue = 0x01,
    Green = 0x02,
    Cyan = 0x03,
    Red = 0x04,
    Magenta = 0x05,
    Brown = 0x06,
    LightGrey = 0x07,
    DarkGrey = 0x08,
    LightBlue = 0x09,
    LightGreen = 0x0A,
    LightCyan = 0x0B,
    LightRed = 0x0C,
    LightMagenta = 0x0D,
    LightBrown = 0x0E,
    White = 0x0F,
}

pub struct Writer {
    buf: &'static mut [Volatile<u16>],
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
    /// Returns position of most recently written character
    pub fn write_byte(&mut self, ch: u8) -> u16 {
        if self.handle_escapes(ch) {
            return 0;
        }

        self.handle_scrolling();

        // Actually write the character to the screen, escapes have been handled
        // previously no need to worry about those anymore.
        let pos: u16 = (self.y * WIDTH + self.x) as u16;
        let byte: u16 = ((Color::LightGreen as u16) << 8) | u16::from(ch);
        self.buf[self.y * WIDTH + self.x].write(byte);

        self.update_cursor_position();

        pos
    }

    pub fn write_string(&mut self, s: &str) {
        // If dbg, write string to serial port first
        #[cfg(debug_assertions)]
        serial::write_string(s);

        let mut pos: u16 = 0;
        for byte in s.bytes() {
            // TODO: Handle non-ascii chars
            pos = self.write_byte(byte);
        }

        if pos != 0 {
            update_cursor(pos);
        }
    }
}

// Everything below here is just glue for the 'println!, print!, info!, debug!,
// error!, warn!' macros

// Need a separate struct so we can implement Log trait
pub struct SpinLockWriter(SpinLock<Writer>);

pub fn init() -> Result<(), SetLoggerError> {
    // TODO: Refactor this into separate module (how about vga/mod.rs)
    #[cfg(debug_assertions)]
    serial::init();

    enable_cursor();

    log::set_logger(&*WRITER).map(|()| {
        #[cfg(debug_assertions)]
        log::set_max_level(LevelFilter::Debug);

        #[cfg(not(debug_assertions))]
        log::set_max_level(LevelFilter::Info);
    })
}

fn update_cursor(pos: u16) {
    unsafe {
        PortWrite::write_to_port(0x3D4 as u16, 0x0F as u8);
        PortWrite::write_to_port(0x3D5 as u16, (pos & 0xFF) as u16);
        PortWrite::write_to_port(0x3D4 as u16, 0x0E as u8);
        PortWrite::write_to_port(0x3D5 as u16, ((pos >> 8) & 0xFF) as u16);
    }
}

fn enable_cursor() {
    const BEGIN_SCANLINE: u16 = 0;
    const END_SCANLINE: u16 = 15;

    unsafe {
        PortWrite::write_to_port(0x3D4 as u16, 0x0A as u8);
        let old: u16 = PortRead::read_from_port(0x3D5);
        PortWrite::write_to_port(0x3D5 as u16, ((old & 0xC0) | BEGIN_SCANLINE) as u8);

        PortWrite::write_to_port(0x3D4 as u16, 0x0B as u8);
        let old: u16 = PortRead::read_from_port(0x3D5);
        PortWrite::write_to_port(0x3D5 as u16, ((old & 0xE0) | END_SCANLINE) as u8);
    }
}

#[allow(dead_code)]
fn disable_cursor() {
    unsafe {
        PortWrite::write_to_port(0x3D4 as u16, 0x0A as u8);
        PortWrite::write_to_port(0x3D5 as u16, 0x20 as u8);
    }
}

lazy_static! {
    pub static ref WRITER: SpinLockWriter = SpinLockWriter(SpinLock::new(Writer {
        buf: unsafe {
            core::slice::from_raw_parts_mut(TERMINAL_BUFFER as *mut Volatile<u16>, 80 * 25)
        },
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

macro_rules! print {
    ($($arg:tt)*) => ($crate::drivers::vga::text_mode::_print(format_args!($($arg)*)));
}

macro_rules! println {
    () => (print!("\n"));
    ($($arg:tt)*) => (print!("{}\n", format_args!($($arg)*)));
}

// Lifted from standard library
#[allow(unused_macros)]
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
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.0.lock().write_fmt(args).unwrap()
    });
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
