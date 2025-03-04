use abstractions::IUsizeAlias;

use crate::*;

#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualAddress(*const ());

impl_IAddress!(VirtualAddress);

impl VirtualAddress {
    pub fn from_ref<T>(r: &T) -> VirtualAddress {
        VirtualAddress::from_ptr(r as *const T)
    }

    pub fn from_ptr<T>(p: *const T) -> VirtualAddress {
        VirtualAddress::from_usize(p as usize)
    }

    /// # Safety
    /// The caller must ensure that the pointer is valid.
    pub unsafe fn as_ref<T>(&self) -> &'static T {
        &*(self.as_usize() as *const T)
    }

    /// # Safety
    /// The caller must ensure that the pointer is valid.
    pub unsafe fn as_mut<T>(&self) -> &'static mut T {
        &mut *(self.as_usize() as *mut T)
    }

    /// # Safety
    /// The caller must ensure that the pointer is valid.
    pub unsafe fn as_ptr<T>(&self) -> *const T {
        self.as_usize() as *const T
    }

    /// # Safety
    /// The caller must ensure that the pointer is valid.
    pub unsafe fn as_mut_ptr<T>(&self) -> *mut T {
        self.as_usize() as *mut T
    }
}

impl IToPageNum<VirtualPageNum> for VirtualAddress {}

#[cfg(test)]
mod virtual_address_tests {
    use alloc::format;

    use super::*;

    // 基本构造和操作测试
    #[test]
    fn test_virtual_address_creation() {
        let addr = VirtualAddress::from_usize(0x1000);
        assert_eq!(addr.as_usize(), 0x1000);
    }

    // IAddress trait 实现测试
    #[test]
    fn test_add_by() {
        let addr = VirtualAddress::from_usize(0x1000);
        assert_eq!(addr.add_by(0x500).as_usize(), 0x1500);
    }

    #[test]
    fn test_minus_by() {
        let addr = VirtualAddress::from_usize(0x1500);
        assert_eq!(addr.minus_by(0x500).as_usize(), 0x1000);
    }

    #[test]
    fn test_off_by() {
        let addr = VirtualAddress::from_usize(0x1000);
        assert_eq!(addr.off_by(0x500).as_usize(), 0x1500);
        assert_eq!(addr.off_by(-0x500).as_usize(), 0xB00);
    }

    #[test]
    fn test_diff() {
        let addr1 = VirtualAddress::from_usize(0x1500);
        let addr2 = VirtualAddress::from_usize(0x1000);
        assert_eq!(addr1.diff(addr2), 0x500);
    }

    #[test]
    fn test_operators() {
        let mut addr = VirtualAddress::from_usize(0x1000);
        assert_eq!(addr + 0x500, VirtualAddress::from_usize(0x1500));
        assert_eq!(addr - 0x500, VirtualAddress::from_usize(0xB00));

        addr += 0x500;
        assert_eq!(addr, VirtualAddress::from_usize(0x1500));

        addr -= 0x500;
        assert_eq!(addr, VirtualAddress::from_usize(0x1000));
    }

    // 对齐测试
    #[test]
    fn test_alignment() {
        let addr = VirtualAddress::from_usize(0x1234);
        assert!(!addr.is_aligned(0x1000));
        assert_eq!(addr.align_down(0x1000).as_usize(), 0x1000);
        assert_eq!(addr.align_up(0x1000).as_usize(), 0x2000);
    }

    // 边界情况测试
    #[test]
    fn test_zero_address() {
        let addr = VirtualAddress::from_usize(0);
        assert_eq!(addr.as_usize(), 0);
        assert!(addr.is_aligned(1));
    }

    #[test]
    fn test_max_address() {
        let addr = VirtualAddress::from_usize(usize::MAX);
        assert_eq!(addr.as_usize(), usize::MAX);
    }

    // 比较操作测试
    #[test]
    fn test_comparison_operators() {
        let addr1 = VirtualAddress::from_usize(0x1000);
        let addr2 = VirtualAddress::from_usize(0x2000);
        let addr3 = VirtualAddress::from_usize(0x1000);

        assert!(addr1 < addr2);
        assert!(addr2 > addr1);
        assert_eq!(addr1, addr3);
        assert!(addr1 <= addr2);
        assert!(addr2 >= addr1);
        assert!(addr1 <= addr3);
        assert!(addr1 >= addr3);
    }

    // 溢出检查测试
    #[test]
    #[should_panic(expected = "attempt to add with overflow")]
    fn test_add_overflow() {
        let addr = VirtualAddress::from_usize(usize::MAX);
        addr.add_by(1);
    }

    #[test]
    #[should_panic(expected = "attempt to subtract with overflow")]
    fn test_minus_underflow() {
        let addr = VirtualAddress::from_usize(0);
        addr.minus_by(1);
    }

    // Clone 和 Copy trait 测试
    #[test]
    fn test_clone_and_copy() {
        let addr1 = VirtualAddress::from_usize(0x1000);
        let addr2 = addr1; // Copy
        let addr3 = addr1.clone(); // Clone

        assert_eq!(addr1, addr2);
        assert_eq!(addr1, addr3);
    }

    // Debug 和 Display trait 测试
    #[test]
    fn test_debug_and_display() {
        let addr = VirtualAddress::from_usize(0x1234);
        assert_eq!(format!("{:?}", addr), "VirtualAddress(0x1234)");
        assert_eq!(format!("{}", addr), "VirtualAddress(0x1234)");
    }
}
