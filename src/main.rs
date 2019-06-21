#![no_std]
#![no_main]

#[macro_use]
extern crate log;

#[macro_use]
extern crate ddos_drivers as drivers;

extern crate ddos_ds as ds;

use core::panic::PanicInfo;

#[panic_handler]
#[allow(clippy::empty_loop)]
fn panic(_info: &PanicInfo) -> ! {
    error!("{}", _info);
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("HELLO WORLD");
    println!("something that is so much longer than 80 characters aka the width of the console of this project oh, and a few\t1\t2\t3\t4\t5 tabs :)\n1\n2\n3 \\n's aswel :)");
    drivers::vga_text::init().unwrap();

    debug!("test 1");
    info!("test 2");
    warn!("test 3");
    error!("test 4");

    let x = "test 5";
    dbg!(x);

    panic!("Kernel panic!");

    #[allow(clippy::empty_loop)]
    loop {}
}
