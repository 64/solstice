use x86_64::instructions::port::{PortWrite, PortRead};

#[repr(u16)]
enum Port {
    COM1 = 0x3F8,
    COM2 = 0x2F8,
    COM3 = 0x3E8,
    COM4 = 0x2E8,
}

const port: u16 = Port::COM1 as u16;

pub fn init() {

    unsafe {
        PortWrite::write_to_port(port + 1, 0x00 as u8);
        PortWrite::write_to_port(port + 3, 0x80 as u8);
        PortWrite::write_to_port(port + 0, 0x03 as u8);
        PortWrite::write_to_port(port + 1, 0x00 as u8);
        PortWrite::write_to_port(port + 3, 0x03 as u8);
        PortWrite::write_to_port(port + 2, 0xC7 as u8);
        PortWrite::write_to_port(port + 4, 0x0B as u8);
    }
}

fn write_byte(ch: u8) {
    unsafe {
        PortWrite::write_to_port(port as u16, ch);
    }
}

pub fn write_string(s: &str) {
    for byte in s.bytes() {
        write_byte(byte);
    }
}
