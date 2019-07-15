use core::convert::{Into, TryInto};
use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign};

use bit_field::BitField;
use ux::*;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtAddr(usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysAddr(usize);

#[derive(Debug)]
pub struct VirtAddrNotValid(usize);

impl From<PhysAddr> for VirtAddr {
    fn from(phys: PhysAddr) -> VirtAddr {
        VirtAddr::new(phys.as_usize() + 0xFFFF8000_00000000)
    }
}

impl From<VirtAddr> for PhysAddr {
    fn from(virt: VirtAddr) -> PhysAddr {
        PhysAddr::new(virt.as_usize() - 0xFFFF8000_00000000)
    }
}

impl VirtAddr {
    pub fn new(addr: usize) -> VirtAddr {
        Self::try_new(addr).expect(
            "address passed to VirtAddr::new must not contain any data \
             in bits 48 to 64",
        )
    }

    /// Tries to create a new canonical virtual address.
    ///
    /// This function tries to performs sign
    /// extension of bit 47 to make the address canonical. It succeeds if bits 48 to 64 are
    /// either a correct sign extension (i.e. copies of bit 47) or all null. Else, an error
    /// is returned.
    pub fn try_new(addr: usize) -> Result<VirtAddr, VirtAddrNotValid> {
        match addr.get_bits(47..64) {
            0 | 0x1ffff => Ok(VirtAddr(addr)),      // address is canonical
            1 => Ok(VirtAddr::new_unchecked(addr)), // address needs sign extension
            other => Err(VirtAddrNotValid(other)),
        }
    }

    /// Creates a new canonical virtual address without checks.
    ///
    /// This function performs sign extension of bit 47 to make the address canonical, so
    /// bits 48 to 64 are overwritten. If you want to check that these bits contain no data,
    /// use `new` or `try_new`.
    pub fn new_unchecked(mut addr: usize) -> VirtAddr {
        if addr.get_bit(47) {
            addr.set_bits(48..64, 0xffff);
        } else {
            addr.set_bits(48..64, 0);
        }
        VirtAddr(addr)
    }

    /// Creates a virtual address that points to `0`.
    pub const fn zero() -> VirtAddr {
        VirtAddr(0)
    }

    pub fn as_usize(self) -> usize {
        self.0
    }

    /// Creates a virtual address from the given pointer
    pub fn from_ptr<T>(ptr: *const T) -> Self {
        Self::new(ptr as usize)
    }

    /// Converts the address to a raw pointer.
    #[cfg(target_pointer_width = "64")]
    pub fn as_ptr<T>(self) -> *const T {
        cast::usize(self.as_usize()) as *const T
    }

    /// Converts the address to a mutable raw pointer.
    #[cfg(target_pointer_width = "64")]
    pub fn as_mut_ptr<T>(self) -> *mut T {
        self.as_ptr::<T>() as *mut T
    }

    /// Aligns the virtual address upwards to the given alignment.
    ///
    /// See the `align_up` function for more information.
    pub fn align_up<U>(self, align: U) -> Self
    where
        U: Into<usize>,
    {
        VirtAddr(align_up(self.0, align.into()))
    }

    /// Aligns the virtual address downwards to the given alignment.
    ///
    /// See the `align_down` function for more information.
    pub fn align_down<U>(self, align: U) -> Self
    where
        U: Into<usize>,
    {
        VirtAddr(align_down(self.0, align.into()))
    }

    /// Checks whether the virtual address has the demanded alignment.
    pub fn is_aligned<U>(self, align: U) -> bool
    where
        U: Into<usize>,
    {
        self.align_down(align) == self
    }

    /// Returns the 12-bit page offset of this virtual address.
    pub fn page_offset(&self) -> u12 {
        u12::new((self.0 & 0xfff).try_into().unwrap())
    }

    /// Returns the 9-bit level 1 page table index.
    pub fn p1_index(&self) -> u9 {
        u9::new(((self.0 >> 12) & 0o777).try_into().unwrap())
    }

    /// Returns the 9-bit level 2 page table index.
    pub fn p2_index(&self) -> u9 {
        u9::new(((self.0 >> 12 >> 9) & 0o777).try_into().unwrap())
    }

    /// Returns the 9-bit level 3 page table index.
    pub fn p3_index(&self) -> u9 {
        u9::new(((self.0 >> 12 >> 9 >> 9) & 0o777).try_into().unwrap())
    }

    /// Returns the 9-bit level 4 page table index.
    pub fn p4_index(&self) -> u9 {
        u9::new(((self.0 >> 12 >> 9 >> 9 >> 9) & 0o777).try_into().unwrap())
    }
}

impl fmt::Debug for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "VirtAddr({:#x})", self.0)
    }
}

impl Add<usize> for VirtAddr {
    type Output = Self;
    fn add(self, rhs: usize) -> Self::Output {
        VirtAddr::new(self.0 + rhs)
    }
}

impl AddAssign<usize> for VirtAddr {
    fn add_assign(&mut self, rhs: usize) {
        *self = *self + rhs;
    }
}

impl Sub<usize> for VirtAddr {
    type Output = Self;
    fn sub(self, rhs: usize) -> Self::Output {
        VirtAddr::new(self.0.checked_sub(rhs).unwrap())
    }
}

impl SubAssign<usize> for VirtAddr {
    fn sub_assign(&mut self, rhs: usize) {
        *self = *self - rhs;
    }
}

impl Sub<VirtAddr> for VirtAddr {
    type Output = usize;
    fn sub(self, rhs: VirtAddr) -> Self::Output {
        self.as_usize().checked_sub(rhs.as_usize()).unwrap()
    }
}

#[derive(Debug)]
pub struct PhysAddrNotValid(usize);

impl PhysAddr {
    /// Creates a new physical address.
    ///
    /// Panics if a bit in the range 52 to 64 is set.
    pub fn new(addr: usize) -> PhysAddr {
        assert_eq!(
            addr.get_bits(52..64),
            0,
            "physical addresses must not have any bits in the range 52 to 64 set"
        );
        PhysAddr(addr)
    }

    /// Tries to create a new physical address.
    ///
    /// Fails if any bits in the range 52 to 64 are set.
    pub fn try_new(addr: usize) -> Result<PhysAddr, PhysAddrNotValid> {
        match addr.get_bits(52..64) {
            0 => Ok(PhysAddr(addr)), // address is valid
            other => Err(PhysAddrNotValid(other)),
        }
    }

    /// Converts the address to an `usize`.
    pub fn as_usize(self) -> usize {
        self.0
    }

    /// Convenience method for checking if a physical address is null.
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }

    /// Aligns the physical address upwards to the given alignment.
    ///
    /// See the `align_up` function for more information.
    pub fn align_up<U>(self, align: U) -> Self
    where
        U: Into<usize>,
    {
        PhysAddr(align_up(self.0, align.into()))
    }

    /// Aligns the physical address downwards to the given alignment.
    ///
    /// See the `align_down` function for more information.
    pub fn align_down<U>(self, align: U) -> Self
    where
        U: Into<usize>,
    {
        PhysAddr(align_down(self.0, align.into()))
    }

    /// Checks whether the physical address has the demanded alignment.
    pub fn is_aligned<U>(self, align: U) -> bool
    where
        U: Into<usize>,
    {
        self.align_down(align) == self
    }
}

impl fmt::Debug for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PhysAddr({:#x})", self.0)
    }
}

impl fmt::Binary for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::LowerHex for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Octal for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::UpperHex for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Add<usize> for PhysAddr {
    type Output = Self;
    fn add(self, rhs: usize) -> Self::Output {
        PhysAddr::new(self.0 + rhs)
    }
}

impl AddAssign<usize> for PhysAddr {
    fn add_assign(&mut self, rhs: usize) {
        *self = *self + rhs;
    }
}

impl Sub<usize> for PhysAddr {
    type Output = Self;
    fn sub(self, rhs: usize) -> Self::Output {
        PhysAddr::new(self.0.checked_sub(rhs).unwrap())
    }
}

impl SubAssign<usize> for PhysAddr {
    fn sub_assign(&mut self, rhs: usize) {
        *self = *self - rhs;
    }
}

impl Sub<PhysAddr> for PhysAddr {
    type Output = usize;
    fn sub(self, rhs: PhysAddr) -> Self::Output {
        self.as_usize().checked_sub(rhs.as_usize()).unwrap()
    }
}

/// Align address downwards.
///
/// Returns the greatest x with alignment `align` so that x <= addr. The alignment must be
///  a power of 2.
pub fn align_down(addr: usize, align: usize) -> usize {
    assert!(align.is_power_of_two(), "`align` must be a power of two");
    addr & !(align - 1)
}

/// Align address upwards.
///
/// Returns the smallest x with alignment `align` so that x >= addr. The alignment must be
/// a power of 2.
pub fn align_up(addr: usize, align: usize) -> usize {
    assert!(align.is_power_of_two(), "`align` must be a power of two");
    let align_mask = align - 1;
    if addr & align_mask == 0 {
        addr // already aligned
    } else {
        (addr | align_mask) + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_align_up() {
        // align 1
        assert_eq!(align_up(0, 1), 0);
        assert_eq!(align_up(1234, 1), 1234);
        assert_eq!(align_up(0xffffffffffffffff, 1), 0xffffffffffffffff);
        // align 2
        assert_eq!(align_up(0, 2), 0);
        assert_eq!(align_up(1233, 2), 1234);
        assert_eq!(align_up(0xfffffffffffffffe, 2), 0xfffffffffffffffe);
        // address 0
        assert_eq!(align_up(0, 128), 0);
        assert_eq!(align_up(0, 1), 0);
        assert_eq!(align_up(0, 2), 0);
        assert_eq!(align_up(0, 0x8000000000000000), 0);
    }
}
