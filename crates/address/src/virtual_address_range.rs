use abstractions::IUsizeAlias;

use crate::*;

pub type VirtualAddressRange = AddressRange<VirtualAddress>;

impl_range_display!(VirtualAddressRange);

impl VirtualAddressRange {
    pub fn from_slice<T>(slice: &[T]) -> Self {
        let start = VirtualAddress::from_usize(slice.as_ptr() as usize);
        let end = VirtualAddress::from_usize(start.as_usize() + core::mem::size_of_val(slice));
        VirtualAddressRange::from_start_end(start, end)
    }
}

#[cfg(test)]
mod virtual_address_range_tests {
    use abstractions::IUsizeAlias;

    const VIRT_ADDR_OFFSET: usize = 0xffff_ffc0_0000_0000;

    use super::*;

    // 基本构造和操作测试
    #[test]
    fn test_virtual_address_range_creation() {
        let start = VirtualAddress::from_usize(0x1000);
        let end = VirtualAddress::from_usize(0x2000);
        let range = VirtualAddressRange::from_start_end(start, end);
        assert_eq!(range.start().as_usize(), 0x1000);
        assert_eq!(range.end().as_usize(), 0x2000);
    }

    // Identity mapping 测试
    #[test]
    fn test_identity_mapped() {
        let start = VirtualAddress::from_usize(0x1000 | VIRT_ADDR_OFFSET);
        let end = VirtualAddress::from_usize(0x2000 | VIRT_ADDR_OFFSET);
        let range = VirtualAddressRange::from_start_end(start, end);
        let phys_range = range.to_low_physical();
        assert_eq!(phys_range.start().as_usize(), 0x1000);
        assert_eq!(phys_range.end().as_usize(), 0x2000);
    }

    // 范围包含测试
    #[test]
    fn test_contains() {
        let range = VirtualAddressRange::from_start_end(
            VirtualAddress::from_usize(0x1000),
            VirtualAddress::from_usize(0x2000),
        );
        let addr_inside = VirtualAddress::from_usize(0x1800);
        let addr_outside = VirtualAddress::from_usize(0x2000);
        assert!(range.contains(addr_inside));
        assert!(!range.contains(addr_outside));
    }

    // 范围相交测试
    #[test]
    fn test_intersects() {
        let range1 = VirtualAddressRange::from_start_end(
            VirtualAddress::from_usize(0x1000),
            VirtualAddress::from_usize(0x3000),
        );
        let range2 = VirtualAddressRange::from_start_end(
            VirtualAddress::from_usize(0x2000),
            VirtualAddress::from_usize(0x4000),
        );
        assert!(range1.intersects(&range2));
        let intersection = range1.intersection(&range2).unwrap();
        assert_eq!(intersection.start().as_usize(), 0x2000);
        assert_eq!(intersection.end().as_usize(), 0x3000);
    }

    // 范围并集测试
    #[test]
    fn test_union() {
        let range1 = VirtualAddressRange::from_start_end(
            VirtualAddress::from_usize(0x1000),
            VirtualAddress::from_usize(0x2000),
        );
        let range2 = VirtualAddressRange::from_start_end(
            VirtualAddress::from_usize(0x1500),
            VirtualAddress::from_usize(0x2500),
        );
        let union = range1.union(&range2);
        assert_eq!(union.start().as_usize(), 0x1000);
        assert_eq!(union.end().as_usize(), 0x2500);
    }

    // 范围迭代测试
    #[test]
    fn test_iter() {
        let range = VirtualAddressRange::from_start_end(
            VirtualAddress::from_usize(0x1000),
            VirtualAddress::from_usize(0x1003),
        );
        let mut iter = range.iter();
        assert_eq!(iter.next().unwrap().as_usize(), 0x1000);
        assert_eq!(iter.next().unwrap().as_usize(), 0x1001);
        assert_eq!(iter.next().unwrap().as_usize(), 0x1002);
        assert!(iter.next().is_none());
    }

    // 范围是否为空测试
    #[test]
    fn test_is_empty() {
        let range = VirtualAddressRange::from_start_end(
            VirtualAddress::from_usize(0x1000),
            VirtualAddress::from_usize(0x1000),
        );
        assert!(range.is_empty());
    }

    // 范围包含另一个范围测试
    #[test]
    fn test_contains_range() {
        let range1 = VirtualAddressRange::from_start_end(
            VirtualAddress::from_usize(0x1000),
            VirtualAddress::from_usize(0x2000),
        );
        let range2 = VirtualAddressRange::from_start_end(
            VirtualAddress::from_usize(0x1500),
            VirtualAddress::from_usize(0x1800),
        );
        assert!(range1.contains_range(&range2));
        assert!(range2.contained_by(&range1));
    }

    // 范围长度测试
    #[test]
    fn test_len() {
        let range = VirtualAddressRange::from_start_end(
            VirtualAddress::from_usize(0x1000),
            VirtualAddress::from_usize(0x2000),
        );
        assert_eq!(range.len(), 0x1000);
    }

    // 范围起始地址和结束地址测试
    #[test]
    fn test_start_end() {
        let range = VirtualAddressRange::from_start_end(
            VirtualAddress::from_usize(0x1000),
            VirtualAddress::from_usize(0x2000),
        );
        assert_eq!(range.start().as_usize(), 0x1000);
        assert_eq!(range.end().as_usize(), 0x2000);
    }
}
