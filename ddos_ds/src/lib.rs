#![no_std]

pub mod sync;

pub use sync::SpinLock;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
