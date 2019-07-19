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
    fn write_byte(&mut self, ch: u8)  {
        if let Some(ch) = self.state.ransid_process(ch) {
            match ch.ascii {
                b'\n' => self.newline(),
                b'\t' => self.write_str("    "),
                b'\r' => self.x = 0,
                ASCII_MIN..=ASCII_MAX => self.draw_char(ch.style, ch.ascii),
                _ => self.draw_char(ch.style, 254),
            }
        }
    }

    pub fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }

        self.update_cursor();
    }

    fn draw_char(&mut self, style: u8, ch: u8) {
        let formatted = (u16::from(style) << 8) | u16::from(ch);
        self.buf[self.y * WIDTH + self.x].write(formatted);

        if self.x == WIDTH - 1 {
            self.newline();
        } else {
            self.x += 1;
        }
    }

    fn update_cursor(&self) {
        //let pos = self.y * WIDTH + self.x;
        let pos = self.y * WIDTH + self.x;

        unsafe {
            PortWrite::write_to_port(0x3D4, 0x0Fu8);
            PortWrite::write_to_port(0x3D5, (pos & 0xFF) as u16);
            PortWrite::write_to_port(0x3D4, 0x0Eu8);
            PortWrite::write_to_port(0x3D5, ((pos >> 8) & 0xFF) as u16);
        }
    }

    fn newline(&mut self) {
        if self.y == HEIGHT - 1 {
            // Scroll
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
        } else {
            self.y += 1;
        }

        self.x = 0;
    }
}

impl Default for Writer {
    fn default() -> Self {
        Writer {
            state: RansidState::new(),
            buf: unsafe {
                core::slice::from_raw_parts_mut(
                    TERMINAL_BUFFER as *mut Volatile<u16>,
                    HEIGHT * WIDTH,
                )
            },
            x: 0,
            y: 0,
        }
    }
}

pub fn init() -> Result<(), SetLoggerError> {
    // Enable cursor
    const BEGIN_SCANLINE: u16 = 0;
    const END_SCANLINE: u16 = 15;

    unsafe {
        PortWrite::write_to_port(0x3D4u16, 0x0Au8);
        let old: u16 = PortRead::read_from_port(0x3D5);
        PortWrite::write_to_port(0x3D5u16, ((old & 0xC0) | BEGIN_SCANLINE) as u8);

        PortWrite::write_to_port(0x3D4u16, 0x0Bu8);
        let old: u16 = PortRead::read_from_port(0x3D5);
        PortWrite::write_to_port(0x3D5u16, ((old & 0xE0) | END_SCANLINE) as u8);
    }

    // Allows use of logging macros
    log::set_logger(&*macros::SCREEN).map(|()| {
        #[cfg(debug_assertions)]
        log::set_max_level(LevelFilter::Debug);

        #[cfg(not(debug_assertions))]
        log::set_max_level(LevelFilter::Info);
    })
}
