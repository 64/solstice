//! Provides boot information to the kernel.

#![deny(improper_ctypes)]

pub use self::memory_map::*;

mod memory_map;

/// This structure represents the information that the bootloader passes to the
/// kernel.
///
/// The information is passed as an argument to the entry point:
///
/// ```ignore
/// pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
///    // […]
/// }
/// ```
///
/// Note that no type checking occurs for the entry point function, so be
/// careful to use the correct argument types. To ensure that the entry point
/// function has the correct signature, use the [`entry_point`] macro.
#[derive(Debug)]
#[repr(C)]
pub struct BootInfo {
    pub memory_map: MemoryMap,
    pub p4_addr: u64,
    pub physical_memory_offset: u64,
    _non_exhaustive: u8, // `()` is not FFI safe
}

impl BootInfo {
    /// Create a new boot information structure. This function is only for
    /// internal purposes.
    #[allow(unused_variables)]
    #[doc(hidden)]
    pub fn new(memory_map: MemoryMap, p4_addr: u64, physical_memory_offset: u64) -> Self {
        BootInfo {
            memory_map,
            p4_addr,
            physical_memory_offset,
            _non_exhaustive: 0,
        }
    }
}

extern "C" {
    fn _improper_ctypes_check(_boot_info: BootInfo);
}
