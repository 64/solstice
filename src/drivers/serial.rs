#![allow(unused)]
use x86_64::instructions::port::PortWrite;

#[repr(u16)]
#[allow(unused)]
enum Port {
    COM1 = 0x3F8,
    COM2 = 0x2F8,
    COM3 = 0x3E8,
    COM4 = 0x2E8,
}

const PORT: u16 = Port::COM1 as u16;

pub fn init() {
    #[allow(clippy::identity_op)]
    unsafe {
        PortWrite::write_to_port(PORT + 1, 0x00 as u8);
        PortWrite::write_to_port(PORT + 3, 0x80 as u8);
        PortWrite::write_to_port(PORT + 0, 0x03 as u8);
        PortWrite::write_to_port(PORT + 1, 0x00 as u8);
        PortWrite::write_to_port(PORT + 3, 0x03 as u8);
        PortWrite::write_to_port(PORT + 2, 0xC7 as u8);
        PortWrite::write_to_port(PORT + 4, 0x0B as u8);
    }
}

fn write_byte(ch: u8) {
    unsafe {
        PortWrite::write_to_port(PORT as u16, ch);
    }
}

pub fn write_str(s: &str) {
    for byte in s.bytes() {
        write_byte(byte);
    }
}
