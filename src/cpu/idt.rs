use x86_64::structures::idt;
use lazy_static::lazy_static;

lazy_static! {
    static ref IDT: idt::InterruptDescriptorTable = {
        let mut idt = idt::InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt
    };
}

pub fn load() {
    IDT.load();
    info!("IDT loaded");
}

extern "x86-interrupt" fn breakpoint_handler(frame: &mut idt::InterruptStackFrame) {
    warn!("EXCEPTION: Breakpoint\n{:#?}", frame);
}