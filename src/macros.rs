// TODO: Move into macros/ folder

use crate::{drivers::vga::text_mode::Writer, ds::SpinLock};
use core::fmt;
use lazy_static::lazy_static;
use log::{Level, Log, Metadata, Record};

// Need a separate struct so we can implement Log trait
pub struct ScreenLocker(SpinLock<ScreenWriter>);

pub struct ScreenWriter(Writer);

impl fmt::Write for ScreenWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        #[cfg(debug_assertions)]
        {
            use crate::drivers::serial;
            serial::write_str(s);
        }

        self.0.write_str(s);

        Ok(())
    }
}

lazy_static! {
    pub static ref SCREEN: ScreenLocker =
        ScreenLocker(SpinLock::new(ScreenWriter(Writer::default())));
}

macro_rules! print {
    ($($arg:tt)*) => ($crate::macros::_print(format_args!($($arg)*)));
}

macro_rules! println {
    () => (print!("\n"));
    ($($arg:tt)*) => (print!("{}\n", format_args!($($arg)*)));
}

// Lifted from standard library
#[allow(unused_macros)]
macro_rules! dbg {
    () => {
        println!("[DEBUG {}:{}]", file!(), line!());
    };
    ($val:expr) => {
        // Use of `match` here is intentional because it affects the lifetimes
        // of temporaries - https://stackoverflow.com/a/48732525/1063961
        match $val {
            tmp => {
                println!(
                    "[\x1B[36mDEBUG\x1B[0m {}:{}] {} = {:#?}",
                    file!(),
                    line!(),
                    stringify!($val),
                    &tmp
                );
                tmp
            }
        }
    };
    ($val:expr,) => { dbg!($val) };
    ($($val:expr),+ $(,)?) => {
        ($(dbg!($val)),+,)
    };
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    x86_64::instructions::interrupts::without_interrupts(|| {
        SCREEN.0.lock().write_fmt(args).unwrap();
    });
}

impl Log for ScreenLocker {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let color = match record.level() {
                Level::Info => "\x1B[32m",
                Level::Error => "\x1B[31m",
                Level::Warn => "\x1B[33m",
                Level::Debug => "\x1B[36m",
                Level::Trace => "\x1B[35m",
            };

            let reset = "\x1B[0m";

            println!("[{}{}{}] {}", color, record.level(), reset, record.args());
        }
    }

    fn flush(&self) {}
}

macro_rules! test_case {
    ($test_name:ident, $body:expr) => {
        #[test_case]
        fn $test_name() {
            print!("{}::{}... ", module_path!(), stringify!($test_name));
            $body;
            println!("[ok]");
        }
    };
}
