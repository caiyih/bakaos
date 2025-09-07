use abstractions::IUsizeAlias;

use crate::*;

#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalAddress(*const ());

impl_IAddress!(PhysicalAddress);

impl IToPageNum<PhysicalPageNum> for PhysicalAddress {}

#[cfg(test)]
mod physical_address_tests {
    use alloc::format;

    use super::*;

    const VIRT_ADDR_OFFSET: usize = 0xFFFF_FFC0_0000_0000;

    impl const IConvertablePhysicalAddress for PhysicalAddress {
        fn to_high_virtual(&self) -> VirtualAddress {
            VirtualAddress::from_usize(self.as_usize() | VIRT_ADDR_OFFSET)
        }

        fn as_virtual(addr: usize) -> usize {
            addr | VIRT_ADDR_OFFSET
        }

        fn is_valid_pa(_addr: usize) -> bool {
            true
        }
    }

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

    // to virtual address测试
    #[test]
    fn test_to_virtual() {
        let phys_addr = PhysicalAddress::from_usize(0x1000);
        let virt_addr = phys_addr.to_high_virtual();
        assert_eq!(
            phys_addr.as_usize() | VIRT_ADDR_OFFSET,
            virt_addr.as_usize()
        );
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
        let addr3 = addr1; // Clone

        assert_eq!(addr1, addr2);
        assert_eq!(addr1, addr3);
    }

    // Debug 和 Display trait 测试
    #[test]
    fn test_debug_and_display() {
        let addr = PhysicalAddress::from_usize(0x1234);
        assert_eq!(format!("{:?}", addr), "PhysicalAddress(0x1234)");
        assert_eq!(format!("{}", addr), "PhysicalAddress(0x1234)");
    }
}
