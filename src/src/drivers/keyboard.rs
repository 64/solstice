use x86_64::instructions::port::{PortRead, PortWrite};
use crate::drivers::keyboard::Ports::STATUS_COMMAND;
use crate::drivers::keyboard::StatusMasks::{INBUF_STATUS, OUTBUF_STATUS};
use x86_64::structures::idt;
//for now, we're just going to support one layout
#[allow(non_camel_case_types)]
enum Ports {
    DATA = 0x60,
    //status register when read, command register when written to
    STATUS_COMMAND = 0x64
}
#[allow(dead_code)]
#[allow(non_camel_case_types)]
enum StatusMasks {
    OUTBUF_STATUS = 0x01,
    INBUF_STATUS = 0x02,
    SYSFLAG = 0x04,
    COMM_DATA = 0x08,
    UNK1 = 0x10,
    UNK2 = 0x20,
    TIMEOUT_ERR = 0x40,
    PARITY_ERR = 0x80
}
unsafe fn keyboard_output_withwait(p:Ports, data:u8) {
    loop {
        let sbyte:u8 = PortRead::read_from_port(STATUS_COMMAND as u16);
        if (sbyte & (INBUF_STATUS as u8)) == 0 {
            break;
        }
    }
    PortWrite::write_to_port(p as u16,data);
}
unsafe fn keyboard_input_withwait() -> u8 {
    loop {
        let sbyte:u8 = PortRead::read_from_port(STATUS_COMMAND as u16);
        if (sbyte & (OUTBUF_STATUS as u8)) != 0 {
            break;
        }
    }
    return PortRead::read_from_port(Ports::DATA as u16);
}
pub fn init() {
    unsafe {
        keyboard_output_withwait(STATUS_COMMAND, 0xAE);
        keyboard_output_withwait(STATUS_COMMAND, 0x20);
        let mut response_byte:u8 = keyboard_input_withwait();
        response_byte |= 1;
        keyboard_output_withwait(STATUS_COMMAND , 0x60);
        keyboard_output_withwait(Ports::DATA, response_byte);
    }
}
#[allow(unused_variables)]
pub extern "x86-interrupt" fn keyboard_interrupt_handler(frame: &mut idt::InterruptStackFrame) {
    unsafe {
        let incoming_byte: u8 = keyboard_input_withwait();
        info!("Key recieved: {}", incoming_byte);
    }
}