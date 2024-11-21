use crate::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalPageNum(pub usize);

impl_IPageNum!(PhysicalPageNum);

impl PhysicalPageNum {
    pub fn identity_mapped(self) -> VirtualPageNum {
        VirtualPageNum::from_usize(self.as_usize())
    }
}

#[cfg(test)]
mod physical_page_num_tests {
    use super::*;

    // 基本构造和转换测试
    #[test]
    fn test_basic_construction() {
        let page_num = PhysicalPageNum::from_usize(5);
        assert_eq!(page_num.as_usize(), 5);
    }

    // Identity mapping 测试
    #[test]
    fn test_identity_mapped() {
        let phys_page = PhysicalPageNum::from_usize(0x1000);
        let virt_page = phys_page.identity_mapped();
        assert_eq!(phys_page.as_usize(), virt_page.as_usize());
    }

    // 运算符测试
    #[test]
    fn test_arithmetic_operations() {
        let mut page = PhysicalPageNum::from_usize(100);

        // Add/Sub with usize
        assert_eq!((page + 50).as_usize(), 150);
        assert_eq!((page - 50).as_usize(), 50);

        // Add/Sub with same type
        let other = PhysicalPageNum::from_usize(50);
        assert_eq!((page + other).as_usize(), 150);
        assert_eq!((page - other).as_usize(), 50);

        // AddAssign/SubAssign with usize
        page += 50;
        assert_eq!(page.as_usize(), 150);
        page -= 50;
        assert_eq!(page.as_usize(), 100);

        // AddAssign/SubAssign with same type
        page += other;
        assert_eq!(page.as_usize(), 150);
        page -= other;
        assert_eq!(page.as_usize(), 100);
    }

    // 步进操作测试
    #[test]
    fn test_step_operations() {
        let mut page = PhysicalPageNum::from_usize(100);

        page.step();
        assert_eq!(page.as_usize(), 101);

        page.step_by(10);
        assert_eq!(page.as_usize(), 111);

        page.step_back();
        assert_eq!(page.as_usize(), 110);

        page.step_back_by(10);
        assert_eq!(page.as_usize(), 100);
    }

    // 地址转换测试
    #[test]
    fn test_address_conversions() {
        let addr = PhysicalAddress::from_usize(0x2050);
        let page = PhysicalPageNum::from_addr_floor(addr);
        assert_eq!(page.as_usize(), 2);

        let addr = PhysicalAddress::from_usize(0x2050);
        let page = PhysicalPageNum::from_addr_ceil(addr);
        assert_eq!(page.as_usize(), 3);
    }

    // 地址计算测试
    #[test]
    fn test_address_calculations() {
        let page = PhysicalPageNum::from_usize(0x1000);

        // Start and end addresses
        let start_addr: PhysicalAddress = page.start_addr();
        let end_addr: PhysicalAddress = page.end_addr();
        assert_eq!(start_addr.as_usize(), 0x0100_0000);
        assert_eq!(end_addr.as_usize(), 0x0100_1000);

        // Offset calculations
        let offset_addr: PhysicalAddress = page.at_offset_of_start(0x500);
        assert_eq!(offset_addr.as_usize(), 0x0100_0500);

        let offset_addr: PhysicalAddress = page.at_offset_of_end(0x500);
        assert_eq!(offset_addr.as_usize(), 0x0100_0B00);

        let addr = start_addr + 0x500;

        assert_eq!(page.start_offset_of_addr(addr), 0x500);
        assert_eq!(page.end_offset_of_addr(addr), 0x500 - 0x1000);
    }

    // 比较操作测试
    #[test]
    fn test_comparison_operations() {
        let page1 = PhysicalPageNum::from_usize(100);
        let page2 = PhysicalPageNum::from_usize(200);
        let page3 = PhysicalPageNum::from_usize(100);

        assert!(page1 < page2);
        assert!(page2 > page1);
        assert_eq!(page1, page3);
        assert!(page1 <= page2);
        assert!(page2 >= page1);
    }

    // 差值计算测试
    #[test]
    fn test_diff_calculations() {
        let page1 = PhysicalPageNum::from_usize(200);
        let page2 = PhysicalPageNum::from_usize(100);

        assert_eq!(page1.diff_page_count(page2), 100);
    }

    // 地址范围测试
    #[test]
    fn test_address_range() {
        let page = PhysicalPageNum::from_usize(0x1000);
        let range: AddressRange<PhysicalAddress> = page.addr_range();

        assert_eq!(range.start().as_usize(), 0x0100_0000);
        assert_eq!(range.end().as_usize(), 0x0100_1000);
    }

    // 溢出测试
    #[test]
    #[should_panic(expected = "attempt to add with overflow")]
    fn test_add_overflow() {
        let page = PhysicalPageNum::from_usize(usize::MAX);
        let _ = page + 1;
    }

    #[test]
    #[should_panic(expected = "attempt to subtract with overflow")]
    fn test_sub_underflow() {
        let page = PhysicalPageNum::from_usize(0);
        let _ = page - 1;
    }

    // Clone 和 Copy trait 测试
    #[test]
    fn test_clone_and_copy() {
        let page1 = PhysicalPageNum::from_usize(100);
        let page2 = page1; // Copy
        let page3 = page1.clone(); // Clone

        assert_eq!(page1, page2);
        assert_eq!(page1, page3);
    }

    // Debug 和 Display trait 测试
    #[test]
    fn test_debug_and_display() {
        let page = PhysicalPageNum::from_usize(0x1234);
        assert_eq!(format!("{:?}", page), "PhysicalPageNum(4660)");
        assert_eq!(format!("{}", page), "PhysicalPageNum(0x1234)");
    }
}
