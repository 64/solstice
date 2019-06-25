#![allow(unused_imports)]
use crate::qemu;

use core::panic::PanicInfo;

#[panic_handler]
#[cfg(test)]
fn panic(info: &PanicInfo) -> ! {
    println!("[failed] {}", info);
    qemu::exit(qemu::ExitCode::Failure);
    loop {}
}

#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn()]) {
    info!("Running {} tests", tests.len());
    println!("-----------------------");

    for test in tests {
        test();
    }

    qemu::exit(qemu::ExitCode::Success);
}

// Example test
test_case!(basic_test, {
    assert_eq!(1, 1);
});
