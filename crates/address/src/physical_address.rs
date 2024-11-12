use crate::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalAddress(usize);

impl_IAddress!(PhysicalAddress);

impl PhysicalAddress {
    pub fn identity_mapped(self) -> VirtualAddress {
        VirtualAddress::from_usize(self.as_usize())
    }
}

// Only implement `as_ref` and `as_mut` for `PhysicalAddress`
// as virtual address may not be identity mapped.

impl PhysicalAddress {
    pub fn from_ptr<T>(ptr: *const T) -> Self {
        Self::from_usize(ptr as usize)
    }

    pub fn from_ref<T>(r: &T) -> Self {
        Self::from_ptr(r as *const T)
    }

    pub fn as_ptr<T>(self) -> *const T {
        self.as_usize() as *const T
    }

    pub fn as_mut_ptr<T>(self) -> *mut T {
        self.as_usize() as *mut T
    }

    // Supress warning: warning: method `as_ref` can be confused for the standard trait method `std::convert::AsRef::as_ref`
    #[allow(clippy::should_implement_trait)]
    pub fn as_ref<T>(self) -> &'static T {
        unsafe { &*(self.as_usize() as *const T) }
    }

    pub fn as_mut<T>(self) -> &'static mut T {
        unsafe { &mut *(self.as_usize() as *mut T) }
    }

    pub fn as_slice<T>(self, len: usize) -> &'static [T] {
        unsafe { core::slice::from_raw_parts(self.as_usize() as *const T, len) }
    }

    pub fn as_mut_slice<T>(self, len: usize) -> &'static mut [T] {
        unsafe { core::slice::from_raw_parts_mut(self.as_usize() as *mut T, len) }
    }
}

#[cfg(test)]
mod physical_address_tests {
    use super::*;

    // 基本构造和操作测试
    #[test]
    fn test_physical_address_creation() {
        let addr = PhysicalAddress::from_usize(0x1000);
        assert_eq!(addr.as_usize(), 0x1000);
    }

    // IAddress trait 实现测试
    #[test]
    fn test_add_by() {
        let addr = PhysicalAddress::from_usize(0x1000);
        assert_eq!(addr.add_by(0x500).as_usize(), 0x1500);
    }

    #[test]
    fn test_minus_by() {
        let addr = PhysicalAddress::from_usize(0x1500);
        assert_eq!(addr.minus_by(0x500).as_usize(), 0x1000);
    }

    #[test]
    fn test_off_by() {
        let addr = PhysicalAddress::from_usize(0x1000);
        assert_eq!(addr.off_by(0x500).as_usize(), 0x1500);
        assert_eq!(addr.off_by(-0x500).as_usize(), 0xB00);
    }

    #[test]
    fn test_diff() {
        let addr1 = PhysicalAddress::from_usize(0x1500);
        let addr2 = PhysicalAddress::from_usize(0x1000);
        assert_eq!(addr1.diff(addr2), 0x500);
    }

    #[test]
    fn test_operators() {
        let mut addr = PhysicalAddress::from_usize(0x1000);
        assert_eq!(addr + 0x500, PhysicalAddress::from_usize(0x1500));
        assert_eq!(addr - 0x500, PhysicalAddress::from_usize(0xB00));

        addr += 0x500;
        assert_eq!(addr, PhysicalAddress::from_usize(0x1500));

        addr -= 0x500;
        assert_eq!(addr, PhysicalAddress::from_usize(0x1000));
    }

    // 对齐测试
    #[test]
    fn test_alignment() {
        let addr = PhysicalAddress::from_usize(0x1234);
        assert!(!addr.is_aligned(0x1000));
        assert_eq!(addr.align_down(0x1000).as_usize(), 0x1000);
        assert_eq!(addr.align_up(0x1000).as_usize(), 0x2000);
    }

    // 边界情况测试
    #[test]
    fn test_zero_address() {
        let addr = PhysicalAddress::from_usize(0);
        assert_eq!(addr.as_usize(), 0);
        assert!(addr.is_aligned(1));
    }

    #[test]
    fn test_max_address() {
        let addr = PhysicalAddress::from_usize(usize::MAX);
        assert_eq!(addr.as_usize(), usize::MAX);
    }

    // Identity mapping 测试
    #[test]
    fn test_identity_mapped() {
        let phys_addr = PhysicalAddress::from_usize(0x1000);
        let virt_addr = phys_addr.identity_mapped();
        assert_eq!(phys_addr.as_usize(), virt_addr.as_usize());
    }

    // 指针和引用转换测试
    #[test]
    fn test_from_ptr() {
        let value = 42;
        let ptr = &value as *const i32;
        let addr = PhysicalAddress::from_ptr(ptr);
        assert_eq!(addr.as_usize(), ptr as usize);
    }

    #[test]
    fn test_from_ref() {
        let value = 42;
        let addr = PhysicalAddress::from_ref(&value);
        assert_eq!(addr.as_usize(), (&value as *const i32) as usize);
    }

    // 比较操作测试
    #[test]
    fn test_comparison_operators() {
        let addr1 = PhysicalAddress::from_usize(0x1000);
        let addr2 = PhysicalAddress::from_usize(0x2000);
        let addr3 = PhysicalAddress::from_usize(0x1000);

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
        let addr = PhysicalAddress::from_usize(usize::MAX);
        addr.add_by(1);
    }

    #[test]
    #[should_panic(expected = "attempt to subtract with overflow")]
    fn test_minus_underflow() {
        let addr = PhysicalAddress::from_usize(0);
        addr.minus_by(1);
    }

    // Clone 和 Copy trait 测试
    #[test]
    fn test_clone_and_copy() {
        let addr1 = PhysicalAddress::from_usize(0x1000);
        let addr2 = addr1; // Copy
        let addr3 = addr1.clone(); // Clone

        assert_eq!(addr1, addr2);
        assert_eq!(addr1, addr3);
    }

    // Debug 和 Display trait 测试
    #[test]
    fn test_debug_and_display() {
        let addr = PhysicalAddress::from_usize(0x1234);
        assert_eq!(format!("{:?}", addr), "PhysicalAddress(4660)");
        assert_eq!(format!("{}", addr), "PhysicalAddress(0x1234)");
    }
}
