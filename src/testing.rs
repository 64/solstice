#![allow(unused_imports)]
use crate::qemu;
use core::panic::PanicInfo;

#[panic_handler]
#[cfg(test)]
fn panic(_info: &PanicInfo) -> ! {
    // TODO: Print panic message to serial
    qemu::exit(qemu::ExitCode::Failure);
    loop {}
}

#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn()]) {
    info!("Running {} tests", tests.len());

    for test in tests {
        test();
    }

    qemu::exit(qemu::ExitCode::Success);
}

#[test_case]
fn trivial_assertion() {
    print!("trivial assertion... ");
    assert_eq!(1, 1);
    println!("[ok]");
}
