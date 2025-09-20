macro_rules! impl_page {
    ($page_type:tt, $addr_type:tt, $range_type:tt, $(#[$doc:meta])*) => {
        $(#[$doc])*
        #[repr(C)]
        #[derive(Clone, Copy, Eq)]
        pub struct $page_type {
            addr: $addr_type<'static>,
            size: usize, // TODO: should we use a enum for common sizes?
        }

        impl $page_type {
            /// Standard 4KB page size (4,096 bytes).
            ///
            /// This is the most common page size used in modern operating systems
            /// and corresponds to the smallest page size supported by most processors.
            pub const SIZE_4K: usize = 0x1000;

            /// Standard 2MB page size (2,097,152 bytes).
            ///
            /// This is a large page size that can improve TLB efficiency for
            /// applications that use large amounts of memory. Also known as
            /// "huge pages" on some systems.
            pub const SIZE_2M: usize = 0x200000;

            /// Standard 1GB page size (1,073,741,824 bytes).
            ///
            /// This is the largest standard page size, used for very large memory
            /// mappings to maximize TLB efficiency. Also known as "gigantic pages"
            /// on some systems.
            pub const SIZE_1G: usize = 0x40000000;

            /// Creates a new 4KB page at the specified address.
            ///
            /// This function validates that the address is properly aligned to 4KB boundaries.
            ///
            /// # Parameters
            /// - `addr`: The starting address of the page, must be 4KB-aligned
            ///
            /// # Returns
            /// - `Some(page)` if the address is properly aligned to 4KB
            /// - `None` if the address is not 4KB-aligned
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysAddr};
            /// let page = PhysPage::new_4k(PhysAddr::new(0x1000));
            /// assert!(page.is_some());
            ///
            /// let unaligned = PhysPage::new_4k(PhysAddr::new(0x1001));
            /// assert!(unaligned.is_none());
            /// ```
            #[inline(always)]
            pub const fn new_4k(addr: $addr_type<'static>) -> Option<Self> {
                Self::new_custom(addr, Self::SIZE_4K)
            }

            /// Creates a new 2MB page at the specified address.
            ///
            /// This function validates that the address is properly aligned to 2MB boundaries.
            ///
            /// # Parameters
            /// - `addr`: The starting address of the page, must be 2MB-aligned
            ///
            /// # Returns
            /// - `Some(page)` if the address is properly aligned to 2MB
            /// - `None` if the address is not 2MB-aligned
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysAddr};
            /// let page = PhysPage::new_2m(PhysAddr::new(0x200000));
            /// assert!(page.is_some());
            ///
            /// let unaligned = PhysPage::new_2m(PhysAddr::new(0x100000));
            /// assert!(unaligned.is_none());
            /// ```
            #[inline(always)]
            pub const fn new_2m(addr: $addr_type<'static>) -> Option<Self> {
                Self::new_custom(addr, Self::SIZE_2M)

            }

            /// Creates a new 1GB page at the specified address.
            ///
            /// This function validates that the address is properly aligned to 1GB boundaries.
            ///
            /// # Parameters
            /// - `addr`: The starting address of the page, must be 1GB-aligned
            ///
            /// # Returns
            /// - `Some(page)` if the address is properly aligned to 1GB
            /// - `None` if the address is not 1GB-aligned
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysAddr};
            /// let page = PhysPage::new_1g(PhysAddr::new(0x40000000));
            /// assert!(page.is_some());
            ///
            /// let unaligned = PhysPage::new_1g(PhysAddr::new(0x20000000));
            /// assert!(unaligned.is_none());
            /// ```
            #[inline(always)]
            pub const fn new_1g(addr: $addr_type<'static>) -> Option<Self> {
                Self::new_custom(addr, Self::SIZE_1G)

            }

            /// Creates a new page with a custom size at the specified address.
            ///
            /// This function validates both the size and address alignment. The address
            /// must be aligned to the specified size boundary, and the size must be non-zero.
            ///
            /// # Parameters
            /// - `addr`: The starting address of the page, must be aligned to `size`
            /// - `size`: The size of the page in bytes, must be non-zero
            ///
            /// # Returns
            /// - `Some(page)` if the address is properly aligned and size is valid
            /// - `None` if the address is not aligned to `size` or if `size` is zero
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysAddr};
            /// // Create an 8KB page
            /// let page = PhysPage::new_custom(PhysAddr::new(0x2000), 0x2000);
            /// assert!(page.is_some());
            ///
            /// // Unaligned address fails
            /// let unaligned = PhysPage::new_custom(PhysAddr::new(0x1001), 0x1000);
            /// assert!(unaligned.is_none());
            ///
            /// // Zero size fails
            /// let zero_size = PhysPage::new_custom(PhysAddr::new(0x1000), 0);
            /// assert!(zero_size.is_none());
            /// ```
            #[inline(always)]
            pub const fn new_custom(addr: $addr_type<'static>, size: usize) -> Option<Self> {
                if size != 0 && addr.is_aligned(size) {
                    Some(Self { addr, size })
                } else {
                    None
                }
            }

            /// Creates a new page with a custom size without validation.
            ///
            /// # Safety
            /// This function does not validate address alignment or size. The caller must
            /// ensure proper alignment if required for the intended use case.
            ///
            /// # Parameters
            /// - `addr`: The starting address of the page
            /// - `size`: The size of the page in bytes
            ///
            /// # Returns
            /// A new page object with the specified size
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysAddr};
            /// // Any address and size combination is allowed
            /// let page = PhysPage::new_custom_unchecked(PhysAddr::new(0x1234), 0x5678);
            /// assert_eq!(page.size(), 0x5678);
            /// ```
            #[inline(always)]
            pub const fn new_custom_unchecked(addr: $addr_type<'static>, size: usize) -> Self {
                Self { addr, size }
            }
        }

        impl $page_type {
            /// Returns the starting address of this page.
            ///
            /// # Returns
            /// The address where this page begins in memory
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysAddr};
            /// let page = PhysPage::new_4k(PhysAddr::new(0x1000)).unwrap();
            /// assert_eq!(page.addr(), PhysAddr::new(0x1000));
            /// ```
            #[inline(always)]
            pub const fn addr(&self) -> $addr_type<'static> {
                self.addr
            }

            /// Returns the size of this page in bytes.
            ///
            /// # Returns
            /// The size of this page in bytes
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysAddr};
            /// let page_4k = PhysPage::new_4k(PhysAddr::new(0x1000)).unwrap();
            /// assert_eq!(page_4k.size(), 0x1000);
            ///
            /// let page_2m = PhysPage::new_2m(PhysAddr::new(0x200000)).unwrap();
            /// assert_eq!(page_2m.size(), 0x200000);
            /// ```
            #[inline(always)]
            pub const fn size(&self) -> usize {
                self.size
            }

            /// Converts this page to an address range.
            ///
            /// Creates an address range that spans from the page's starting address
            /// to the end of the page (start address + page size).
            ///
            /// # Returns
            /// An address range covering this entire page
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysAddr};
            /// let page = PhysPage::new_4k(PhysAddr::new(0x1000)).unwrap();
            /// let range = page.as_range();
            /// assert_eq!(range.start(), PhysAddr::new(0x1000));
            /// assert_eq!(range.end(), PhysAddr::new(0x2000));
            /// ```
            #[inline(always)]
            pub const fn as_range(&self) -> $range_type {
                unsafe { $range_type::new_unchecked(self.addr, self.addr + self.size) }
            }

            /// Attempts to create a page from an address range.
            ///
            /// This function validates that the range length is multiple of the
            /// specified page size and that the starting address is properly aligned.
            ///
            /// # Parameters
            /// - `range`: The address range to convert to a page
            /// - `page_size`: The expected page size in bytes
            ///
            /// # Returns
            /// - `Some(page)` if the range length matches `page_size` and the start is aligned
            /// - `None` if the range length doesn't match or the start address is misaligned
            ///
            /// # Examples
            /// ```rust
            /// # use address_v2::{PhysPage, PhysAddr, PhysAddrRange};
            /// let range = PhysAddrRange::new(PhysAddr::new(0x1000), PhysAddr::new(0x2000));
            /// let page = PhysPage::try_from_range(range, 0x1000);
            /// assert!(page.is_some());
            ///
            /// // Wrong size fails
            /// let wrong_size = PhysPage::try_from_range(range, 0x2000);
            /// assert!(wrong_size.is_none());
            /// ```
            #[inline(always)]
            pub const fn try_from_range(range: $range_type, page_size: usize) -> Option<Self> {
                let len = *range.end() - *range.start();

                if !(len.is_multiple_of(page_size)) {
                    None
                } else {
                    $page_type::new_custom(range.start(), page_size)
                }
            }
        }

        impl ::core::fmt::Debug for $page_type {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}({:?}, size: {:#x})", stringify!($page_type), self.addr, self.size)
            }
        }

        impl ::core::fmt::Display for $page_type {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}({:#x}, size: {:#x})", stringify!($page_type), *self.addr, self.size)
            }
        }

        impl const ::core::ops::Add<usize> for $page_type {
            type Output = Self;

            #[inline(always)]
            fn add(self, rhs: usize) -> Self::Output {
                Self {
                    addr: self.addr + rhs * self.size,
                    size: self.size,
                }
            }
        }

        impl ::core::ops::AddAssign<usize> for $page_type {
            #[inline(always)]
            fn add_assign(&mut self, rhs: usize) {
                self.addr += rhs * self.size;
            }
        }

        impl const ::core::ops::Sub<usize> for $page_type {
            type Output = Self;

            #[inline(always)]
            fn sub(self, rhs: usize) -> Self::Output {
                Self {
                    addr: self.addr - rhs * self.size,
                    size: self.size,
                }
            }
        }

        impl ::core::ops::SubAssign<usize> for $page_type {
            #[inline(always)]
            fn sub_assign(&mut self, rhs: usize) {
                self.addr -= rhs * self.size;
            }
        }

        impl const ::core::cmp::PartialEq for $page_type {
            #[inline(always)]
            fn eq(&self, other: &Self) -> bool {
                self.addr == other.addr && self.size == other.size
            }
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            /// Test the 4K page constructors
            #[test]
            fn test_new_4k() {
                // Test aligned address
                let addr = $addr_type::new(0x1000);
                let page = $page_type::new_4k(addr).unwrap();
                assert_eq!(page.addr(), addr);
                assert_eq!(page.size(), $page_type::SIZE_4K);

                // Test page boundaries
                let page_0 = $page_type::new_4k($addr_type::new(0x0)).unwrap();
                assert_eq!(page_0.size(), $page_type::SIZE_4K);

                let page_high = $page_type::new_4k($addr_type::new(0x1000000)).unwrap();
                assert_eq!(page_high.size(), $page_type::SIZE_4K);
            }

            /// Test the 2M page constructors
            #[test]
            fn test_new_2m() {
                // Test aligned address
                let addr = $addr_type::new(0x200000);
                let page = $page_type::new_2m(addr).unwrap();
                assert_eq!(page.addr(), addr);
                assert_eq!(page.size(), $page_type::SIZE_2M);

                // Test zero address
                let page_0 = $page_type::new_2m($addr_type::new(0x0)).unwrap();
                assert_eq!(page_0.size(), $page_type::SIZE_2M);
            }

            /// Test the 1G page constructors
            #[test]
            fn test_new_1g() {
                // Test aligned address
                let addr = $addr_type::new(0x40000000);
                let page = $page_type::new_1g(addr).unwrap();
                assert_eq!(page.addr(), addr);
                assert_eq!(page.size(), $page_type::SIZE_1G);

                // Test zero address
                let page_0 = $page_type::new_1g($addr_type::new(0x0)).unwrap();
                assert_eq!(page_0.size(), $page_type::SIZE_1G);
            }

            /// Test custom page size constructors
            #[test]
            fn test_new_custom() {
                // Test valid custom sizes with aligned addresses
                let page_8k = $page_type::new_custom($addr_type::new(0x2000), 0x2000);
                assert!(page_8k.is_some());
                assert_eq!(page_8k.unwrap().size(), 0x2000);

                let page_64k = $page_type::new_custom($addr_type::new(0x10000), 0x10000);
                assert!(page_64k.is_some());
                assert_eq!(page_64k.unwrap().size(), 0x10000);

                // Test aligned address with power-of-2 size
                let page_1m = $page_type::new_custom($addr_type::new(0x100000), 0x100000);
                assert!(page_1m.is_some());
                assert_eq!(page_1m.unwrap().size(), 0x100000);
            }

            #[test]
            fn test_new_custom_invalid() {
                // Test unaligned address
                let unaligned = $page_type::new_custom($addr_type::new(0x1001), 0x1000);
                assert!(unaligned.is_none());

                // Test zero size
                let zero_size = $page_type::new_custom($addr_type::new(0x1000), 0);
                assert!(zero_size.is_none());

                // Test size that doesn't align with address
                let misaligned = $page_type::new_custom($addr_type::new(0x1000), 0x3000);
                assert!(misaligned.is_none());
            }

            #[test]
            fn test_new_custom_unchecked() {
                // Should work with any address and size
                let page1 = $page_type::new_custom_unchecked($addr_type::new(0x1001), 0x2000);
                assert_eq!(page1.size(), 0x2000);
                assert_eq!(*page1.addr(), 0x1001);

                let page2 = $page_type::new_custom_unchecked($addr_type::new(0x0), 0);
                assert_eq!(page2.size(), 0);
                assert_eq!(*page2.addr(), 0);

                // Large size
                let page3 = $page_type::new_custom_unchecked($addr_type::new(0x80000000), 0x80000000);
                assert_eq!(page3.size(), 0x80000000);
            }

            /// Test accessor methods
            #[test]
            fn test_accessors() {
                let addr = $addr_type::new(0x1000);
                let page = $page_type::new_4k(addr).unwrap();

                // Test addr()
                assert_eq!(page.addr(), addr);
                assert_eq!(*page.addr(), 0x1000);

                // Test size()
                assert_eq!(page.size(), $page_type::SIZE_4K);

                // Test with different page sizes
                let page_2m = $page_type::new_2m($addr_type::new(0x200000)).unwrap();
                assert_eq!(*page_2m.addr(), 0x200000);
                assert_eq!(page_2m.size(), $page_type::SIZE_2M);

                let page_1g = $page_type::new_1g($addr_type::new(0x40000000)).unwrap();
                assert_eq!(*page_1g.addr(), 0x40000000);
                assert_eq!(page_1g.size(), $page_type::SIZE_1G);
            }

            #[test]
            fn test_as_range() {
                // Test 4K page
                let page_4k = $page_type::new_4k($addr_type::new(0x1000)).unwrap();
                let range_4k = page_4k.as_range();
                assert_eq!(*range_4k.start(), 0x1000);
                assert_eq!(*range_4k.end(), 0x1000 + $page_type::SIZE_4K);

                // Test 2M page
                let page_2m = $page_type::new_2m($addr_type::new(0x200000)).unwrap();
                let range_2m = page_2m.as_range();
                assert_eq!(*range_2m.start(), 0x200000);
                assert_eq!(*range_2m.end(), 0x200000 + $page_type::SIZE_2M);

                // Test 1G page
                let page_1g = $page_type::new_1g($addr_type::new(0x40000000)).unwrap();
                let range_1g = page_1g.as_range();
                assert_eq!(*range_1g.start(), 0x40000000);
                assert_eq!(*range_1g.end(), 0x40000000 + $page_type::SIZE_1G);

                // Test custom size
                let page_custom = $page_type::new_custom_unchecked($addr_type::new(0x8000), 0x8000);
                let range_custom = page_custom.as_range();
                assert_eq!(*range_custom.start(), 0x8000);
                assert_eq!(*range_custom.end(), 0x8000 + 0x8000);
            }

            /// Test try_from_range method
            #[test]
            fn test_try_from_range_success() {
                // Test with 4K aligned range
                let start = $addr_type::new(0x1000);
                let end = $addr_type::new(0x2000);
                let range = unsafe { $range_type::new_unchecked(start, end) };
                let page = $page_type::try_from_range(range, $page_type::SIZE_4K);
                assert!(page.is_some());
                let page = page.unwrap();
                assert_eq!(page.addr(), start);
                assert_eq!(page.size(), $page_type::SIZE_4K);

                // Test with 2M aligned range
                let start_2m = $addr_type::new(0x200000);
                let end_2m = $addr_type::new(0x400000);
                let range_2m = unsafe { $range_type::new_unchecked(start_2m, end_2m) };
                let page_2m = $page_type::try_from_range(range_2m, $page_type::SIZE_2M);
                assert!(page_2m.is_some());
                assert_eq!(page_2m.unwrap().size(), $page_type::SIZE_2M);

                // Test with custom size
                let start_custom = $addr_type::new(0x8000);
                let end_custom = $addr_type::new(0x10000);
                let range_custom = unsafe { $range_type::new_unchecked(start_custom, end_custom) };
                let page_custom = $page_type::try_from_range(range_custom, 0x8000);
                assert!(page_custom.is_some());
                assert_eq!(page_custom.unwrap().size(), 0x8000);
            }

            #[test]
            fn test_try_from_range_failure() {
                // Test with range size not matching page size
                let start = $addr_type::new(0x1000);
                let end = $addr_type::new(0x1800); // 0x800 bytes, not 4K
                let range = unsafe { $range_type::new_unchecked(start, end) };
                let page = $page_type::try_from_range(range, $page_type::SIZE_4K);
                assert!(page.is_none());

                // Test with unaligned start address but correct length
                let start_unaligned = $addr_type::new(0x1001);
                let end_unaligned = $addr_type::new(0x2001);
                let range_unaligned = unsafe { $range_type::new_unchecked(start_unaligned, end_unaligned) };
                let page_unaligned = $page_type::try_from_range(range_unaligned, $page_type::SIZE_4K);
                // Should fail because start address is not aligned to page_size
                assert!(page_unaligned.is_none());

                // Test with correct alignment but wrong length
                let start_wrong_len = $addr_type::new(0x2000);
                let end_wrong_len = $addr_type::new(0x2800); // 0x800 bytes
                let range_wrong_len = unsafe { $range_type::new_unchecked(start_wrong_len, end_wrong_len) };
                let page_wrong_len = $page_type::try_from_range(range_wrong_len, $page_type::SIZE_4K);
                assert!(page_wrong_len.is_none());
            }

            /// Test arithmetic operations
            #[test]
            fn test_add_operation() {
                let page = $page_type::new_4k($addr_type::new(0x1000)).unwrap();

                // Test adding 1 page
                let page_plus_1 = page + 1;
                assert_eq!(*page_plus_1.addr(), 0x1000 + $page_type::SIZE_4K);
                assert_eq!(page_plus_1.size(), $page_type::SIZE_4K);

                // Test adding multiple pages
                let page_plus_5 = page + 5;
                assert_eq!(*page_plus_5.addr(), 0x1000 + 5 * $page_type::SIZE_4K);
                assert_eq!(page_plus_5.size(), $page_type::SIZE_4K);

                // Test adding 0
                let page_plus_0 = page + 0;
                assert_eq!(page_plus_0.addr(), page.addr());
                assert_eq!(page_plus_0.size(), page.size());

                // Test with 2M pages
                let page_2m = $page_type::new_2m($addr_type::new(0x200000)).unwrap();
                let page_2m_plus_1 = page_2m + 1;
                assert_eq!(*page_2m_plus_1.addr(), 0x200000 + $page_type::SIZE_2M);
                assert_eq!(page_2m_plus_1.size(), $page_type::SIZE_2M);
            }

            #[test]
            fn test_sub_operation() {
                let page = $page_type::new_4k($addr_type::new(0x5000)).unwrap();

                // Test subtracting 1 page
                let page_minus_1 = page - 1;
                assert_eq!(*page_minus_1.addr(), 0x5000 - $page_type::SIZE_4K);
                assert_eq!(page_minus_1.size(), $page_type::SIZE_4K);

                // Test subtracting multiple pages
                let page_minus_3 = page - 3;
                assert_eq!(*page_minus_3.addr(), 0x5000 - 3 * $page_type::SIZE_4K);
                assert_eq!(page_minus_3.size(), $page_type::SIZE_4K);

                // Test subtracting 0
                let page_minus_0 = page - 0;
                assert_eq!(page_minus_0.addr(), page.addr());
                assert_eq!(page_minus_0.size(), page.size());
            }

            #[test]
            fn test_add_assign() {
                let mut page = $page_type::new_4k($addr_type::new(0x1000)).unwrap();
                let original_addr = *page.addr();

                page += 2;
                assert_eq!(*page.addr(), original_addr + 2 * $page_type::SIZE_4K);
                assert_eq!(page.size(), $page_type::SIZE_4K);

                // Test adding 0
                page += 0;
                assert_eq!(*page.addr(), original_addr + 2 * $page_type::SIZE_4K);
            }

            #[test]
            fn test_sub_assign() {
                let mut page = $page_type::new_4k($addr_type::new(0x5000)).unwrap();
                let original_addr = *page.addr();

                page -= 1;
                assert_eq!(*page.addr(), original_addr - $page_type::SIZE_4K);
                assert_eq!(page.size(), $page_type::SIZE_4K);

                // Test subtracting 0
                page -= 0;
                assert_eq!(*page.addr(), original_addr - $page_type::SIZE_4K);
            }

            /// Test equality and comparison
            #[test]
            fn test_equality() {
                let page1 = $page_type::new_4k($addr_type::new(0x1000));
                let page2 = $page_type::new_4k($addr_type::new(0x1000));
                let page3 = $page_type::new_4k($addr_type::new(0x2000));
                let page4 = $page_type::new_2m($addr_type::new(0x200000)); // Use 2M aligned address

                // Test equality
                assert_eq!(page1, page2);
                assert_ne!(page1, page3); // Different address
                assert_ne!(page1, page4); // Different size

                // Test with custom pages
                let custom1 = $page_type::new_custom_unchecked($addr_type::new(0x8000), 0x8000);
                let custom2 = $page_type::new_custom_unchecked($addr_type::new(0x8000), 0x8000);
                let custom3 = $page_type::new_custom_unchecked($addr_type::new(0x8000), 0x4000);

                assert_eq!(custom1, custom2);
                assert_ne!(custom1, custom3);
            }

            #[test]
            fn test_clone_copy() {
                let page = $page_type::new_4k($addr_type::new(0x1000)).unwrap();

                // Test Clone
                let cloned = page.clone();
                assert_eq!(page, cloned);

                // Test Copy (implicit)
                let copied = page;
                assert_eq!(page, copied);
                // Original should still be usable after copy
                assert_eq!(*page.addr(), 0x1000);
            }

            #[test]
            fn test_debug_formatting() {
                let page_4k = $page_type::new_4k($addr_type::new(0x1000));
                let debug_str = format!("{:?}", page_4k);

                // Should contain the type name, address, and size
                assert!(debug_str.contains(stringify!($page_type)));
                assert!(debug_str.contains("0x1000"));
                assert!(debug_str.contains("size"));

                let page_2m = $page_type::new_2m($addr_type::new(0x200000));
                let debug_str_2m = format!("{:?}", page_2m);
                assert!(debug_str_2m.contains("0x200000"));
            }

            #[test]
            fn test_display_formatting() {
                let page_4k = $page_type::new_4k($addr_type::new(0x1000)).unwrap();
                let display_str = format!("{}", page_4k);

                // Should contain the type name, address in hex, and size in hex
                assert!(display_str.contains(stringify!($page_type)));
                assert!(display_str.contains("0x1000"));
                assert!(display_str.contains("0x1000")); // size: 0x1000

                let page_custom = $page_type::new_custom_unchecked($addr_type::new(0xdeadbeef), 0x8000);
                let display_str_custom = format!("{}", page_custom);
                assert!(display_str_custom.contains("0xdeadbeef"));
                assert!(display_str_custom.contains("0x8000"));
            }

            /// Test edge cases and boundary conditions
            #[test]
            fn test_zero_address() {
                let page_zero = $page_type::new_4k($addr_type::new(0)).unwrap();
                assert_eq!(*page_zero.addr(), 0);
                assert_eq!(page_zero.size(), $page_type::SIZE_4K);

                let range_zero = page_zero.as_range();
                assert_eq!(*range_zero.start(), 0);
                assert_eq!(*range_zero.end(), $page_type::SIZE_4K);
            }

            #[test]
            fn test_maximum_alignment() {
                // Test with large aligned addresses
                let max_addr_4k = usize::MAX & (!($page_type::SIZE_4K - 1)); // Max 4K aligned address

                let page_max = $page_type::new_4k($addr_type::new(max_addr_4k)).unwrap();
                assert_eq!(*page_max.addr(), max_addr_4k);

                // Test with 1G alignment
                let addr_1g = 0x40000000usize;
                let page_1g = $page_type::new_1g($addr_type::new(addr_1g)).unwrap();
                assert_eq!(page_1g.size(), $page_type::SIZE_1G);
            }

            #[test]
            fn test_arithmetic_with_large_numbers() {
                let page = $page_type::new_4k($addr_type::new(0x1000)).unwrap();

                // Test adding large numbers
                let large_add = page + 1000;
                assert_eq!(*large_add.addr(), 0x1000 + 1000 * $page_type::SIZE_4K);

                // Test with custom large size
                let large_page = $page_type::new_custom_unchecked($addr_type::new(0x1000000), 0x100000);
                let large_plus_10 = large_page + 10;
                assert_eq!(*large_plus_10.addr(), 0x1000000 + 10 * 0x100000);
            }

            #[test]
            fn test_size_constants() {
                // Verify the size constants are correct
                assert_eq!($page_type::SIZE_4K, 0x1000);
                assert_eq!($page_type::SIZE_2M, 0x200000);
                assert_eq!($page_type::SIZE_1G, 0x40000000);

                // Verify they are powers of 2
                assert_eq!($page_type::SIZE_4K.count_ones(), 1);
                assert_eq!($page_type::SIZE_2M.count_ones(), 1);
                assert_eq!($page_type::SIZE_1G.count_ones(), 1);

                // Verify relationships
                assert_eq!($page_type::SIZE_2M, $page_type::SIZE_4K * 512);
                assert_eq!($page_type::SIZE_1G, $page_type::SIZE_2M * 512);
            }

            #[test]
            fn test_range_roundtrip() {
                // Test converting page to range and back
                let original_page = $page_type::new_4k($addr_type::new(0x1000)).unwrap();
                let range = original_page.as_range();
                let page_from_range = $page_type::try_from_range(range, $page_type::SIZE_4K);

                assert!(page_from_range.is_some());
                assert_eq!(original_page, page_from_range.unwrap());

                // Test with 2M page
                let original_2m = $page_type::new_2m($addr_type::new(0x200000)).unwrap();
                let range_2m = original_2m.as_range();
                let page_2m_from_range = $page_type::try_from_range(range_2m, $page_type::SIZE_2M);

                assert!(page_2m_from_range.is_some());
                assert_eq!(original_2m, page_2m_from_range.unwrap());
            }

            #[test]
            fn test_custom_size_edge_cases() {
                // Test with size 1 (minimum valid size)
                let page_1 = $page_type::new_custom($addr_type::new(0), 1);
                assert!(page_1.is_some());
                assert_eq!(page_1.unwrap().size(), 1);

                // Test with very large size
                let large_size = 0x80000000usize;
                let page_large = $page_type::new_custom($addr_type::new(0), large_size);
                assert!(page_large.is_some());
                assert_eq!(page_large.unwrap().size(), large_size);

                // Test with non-power-of-2 but aligned size
                let size_3k = 0x3000;
                let page_3k = $page_type::new_custom($addr_type::new(0x3000), size_3k);
                assert!(page_3k.is_some());
                assert_eq!(page_3k.unwrap().size(), size_3k);
            }
        }
    };
}
