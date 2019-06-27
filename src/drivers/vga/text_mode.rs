use crate::{drivers::vga::ransid::RansidState, macros};
use log::{LevelFilter, SetLoggerError};
use volatile::Volatile;
use x86_64::instructions::port::{PortRead, PortWrite};

const TERMINAL_BUFFER: usize = 0xB8000;
const WIDTH: usize = 80;
const HEIGHT: usize = 25;

const ASCII_MAX: u8 = 126;
const ASCII_MIN: u8 = 32;

pub struct Writer {
    state: RansidState,
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
                self.write_str("    ");
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
            // Moves old memory up
            core::intrinsics::volatile_copy_memory(
                self.buf.as_mut_ptr(),
                self.buf[WIDTH..].as_mut_ptr(),
                WIDTH * (HEIGHT - 1),
            );

            // Clears bottom row
            core::intrinsics::volatile_set_memory(
                self.buf[(WIDTH * (HEIGHT - 1))..].as_mut_ptr(),
                0,
                WIDTH,
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
        match self.state.ransid_process(ch) {
            Some(character) => {
                if self.handle_escapes(ch) {
                    return 0;
                }

                self.handle_scrolling();

                // Actually write the character to the screen, escapes have been handled
                // previously no need to worry about those anymore.
                let pos: u16 = (self.y * WIDTH + self.x) as u16;

                // Checks whether ch is a non-ascii character
                // If it's not ascii, sets it to a space (0x20)
                // If it's ascii nothing happens
                let byte: u16;
                if ch > ASCII_MAX || ch < ASCII_MIN {
                    byte = ((character.style as u16) << 8) | u16::from(b' ');
                } else {
                    byte = ((character.style as u16) << 8) | u16::from(character.ascii);
                }

                self.buf[self.y * WIDTH + self.x].write(byte);

                self.update_cursor_position();

                pos
            }
            None => return 0,
        }
    }

    pub fn write_str(&mut self, s: &str) {
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

impl Default for Writer {
    fn default() -> Self {
        Writer {
            state: RansidState::new(),
            buf: unsafe {
                core::slice::from_raw_parts_mut(TERMINAL_BUFFER as *mut Volatile<u16>, 80 * 25)
            },
            x: 0,
            y: 0,
        }
    }
}

pub fn init() -> Result<(), SetLoggerError> {
    enable_cursor();

    log::set_logger(&*macros::SCREEN).map(|()| {
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
