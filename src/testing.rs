#![allow(unused_imports)]
#![allow(dead_code)]
use core::panic::PanicInfo;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
enum ExitCode {
    Success = 0x10,
    Failure = 0x11,
}

fn exit_qemu(exit_code: ExitCode) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0xF4);

    unsafe {
        port.write(exit_code as u32);
    }
}

#[panic_handler]
#[cfg(test)]
fn panic(info: &PanicInfo) -> ! {
    println!("[failed] {}", info);
    exit_qemu(ExitCode::Failure);
    loop {}
}

#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn()]) {
    info!("Running {} tests", tests.len());
    println!("-----------------------");

    for test in tests {
        test();
    }

    exit_qemu(ExitCode::Success);
}

// Example test
test_case!(basic_test, {
    assert_eq!(1, 1);
});
