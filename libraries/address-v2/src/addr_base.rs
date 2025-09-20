/// Macro to implement address types.
macro_rules! impl_addr {
    ($type:tt, $(#[$doc:meta])*) => {
        $(#[$doc])*
        #[repr(transparent)]
        #[derive(Clone, Copy, Eq, PartialOrd, Ord)]
        pub struct $type<'a> {
            _0: usize,
            _marker: ::core::marker::PhantomData<&'a ()>,
        }

        impl $type<'static> {
            /// Returns the null address.
            #[allow(non_upper_case_globals)]
            pub const null: Self = Self::new(0);

            /// Checks if the address is null (0).
            #[inline(always)]
            pub const fn is_null(self) -> bool {
                self == Self::null
            }

            /// Creates a new address from the given `usize` value.
            #[inline(always)]
            pub const fn new(value: usize) -> Self {
                Self {
                    _0: value,
                    _marker: ::core::marker::PhantomData,
                }
            }
        }

        impl $type<'static> {
            /// Aligns the address down to the given alignment.
            ///
            /// # Examples
            /// ```
            /// # use address_v2::PhysAddr;
            /// let addr = PhysAddr::new(0x1234);
            /// let aligned = addr.align_down(0x1000);
            /// assert_eq!(*aligned, 0x1000);
            /// ```
            #[inline(always)]
            pub const fn align_down(mut self, align: usize) -> Self {
                debug_assert!(align != 0);

                // Usually the given align is a constant value
                // By inlining this function, the compiler selects the optimal code path
                // Same for other alignment related functions

                if align.is_power_of_two() {
                    *self &= !(align - 1)
                } else {
                    *self -= (*self % align)
                }

                self
            }

            /// Aligns the address up to the given alignment.
            ///
            /// # Examples
            /// ```
            /// # use address_v2::PhysAddr;
            /// let addr = PhysAddr::new(0x1234);
            /// let aligned = addr.align_up(0x1000);
            /// assert_eq!(*aligned, 0x2000);
            /// ```
            #[inline(always)]
            pub const fn align_up(mut self, align: usize) -> Self {
                debug_assert!(align != 0);

                if align.is_power_of_two() {
                    *self = (*self + align - 1) & !(align - 1)
                } else {
                    *self = (*self).next_multiple_of(align)
                }

                self
            }

            /// Checks if the address is aligned to the given alignment.
            ///
            /// # Examples
            /// ```
            /// # use address_v2::PhysAddr;
            /// let addr = PhysAddr::new(0x1000);
            /// assert!(addr.is_aligned(0x1000));
            /// assert!(!addr.is_aligned(0x2000));
            /// ```
            #[inline(always)]
            pub const fn is_aligned(self, align: usize) -> bool {
                debug_assert!(align != 0);

                if align.is_power_of_two() {
                    (*self & (align - 1)) == 0
                } else {
                    (*self).is_multiple_of(align)
                }
            }
        }

        impl $type<'_> {
            /// Returns the offset from the given alignment.
            ///
            /// # Examples
            /// ```
            /// # use address_v2::PhysAddr;
            /// let addr = PhysAddr::new(0x1234);
            /// assert_eq!(addr.offset_from_alignment(0x1000), 0x234);
            /// ```
            #[inline(always)]
            pub const fn offset_from_alignment(self, align: usize) -> usize {
                debug_assert!(align != 0);

                if align.is_power_of_two() {
                    *self & (align - 1)
                } else {
                    *self % align
                }
            }
        }

        impl const ::core::default::Default for $type<'static> {
            #[inline(always)]
            fn default() -> Self {
                Self::null
            }
        }

        impl const ::core::cmp::PartialEq<usize> for $type<'_> {
            #[inline(always)]
            fn eq(&self, other: &usize) -> bool {
                **self == *other
            }
        }

        impl const ::core::cmp::PartialEq<$type<'_>> for $type<'_> {
            #[inline(always)]
            fn eq(&self, other: &$type) -> bool {
                **self == **other
            }
        }

        impl const ::core::convert::From<usize> for $type<'static> {
            #[inline(always)]
            fn from(value: usize) -> Self {
                Self::new(value)
            }
        }

        impl const ::core::convert::From<$type<'_>> for usize {
            #[inline(always)]
            fn from(value: $type) -> Self {
                value._0
            }
        }

        impl const ::core::ops::Deref for $type<'_> {
            type Target = usize;

            #[inline(always)]
            fn deref(&self) -> &Self::Target {
                &self._0
            }
        }

        impl const ::core::ops::DerefMut for $type<'_> {
            #[inline(always)]
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self._0
            }
        }

        impl ::core::fmt::Display for $type<'_> {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                write!(f, concat!(stringify!($type), "({:#x})"), **self)
            }
        }

        impl ::core::fmt::Debug for $type<'_> {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                write!(f, concat!(stringify!($type), "({:#x})"), **self)
            }
        }

        impl const ::core::ops::Add<usize> for $type<'static> {
            type Output = Self;

            #[inline(always)]
            fn add(mut self, rhs: usize) -> Self::Output {
                *self += rhs;
                self
            }
        }

        impl const ::core::ops::Add<isize> for $type<'static> {
            type Output = Self;

            #[inline(always)]
            fn add(mut self, rhs: isize) -> Self::Output {
                *self = (*self as isize + rhs) as usize;
                self
            }
        }

        impl const ::core::ops::Sub<usize> for $type<'static> {
            type Output = Self;

            #[inline(always)]
            fn sub(mut self, rhs: usize) -> Self::Output {
                *self -= rhs;
                self
            }
        }

        impl const ::core::ops::Sub<isize> for $type<'static> {
            type Output = Self;

            #[inline(always)]
            fn sub(mut self, rhs: isize) -> Self::Output {
                *self = (*self as isize - rhs) as usize;
                self
            }
        }

        impl const ::core::ops::Add<$type<'static>> for $type<'static> {
            type Output = Self;

            #[inline(always)]
            fn add(mut self, rhs: $type) -> Self::Output {
                *self += *rhs;
                self
            }
        }

        impl const ::core::ops::Sub<$type<'static>> for $type<'static> {
            type Output = isize;

            #[inline(always)]
            fn sub(self, rhs: $type) -> Self::Output {
                *self as isize - *rhs as isize
            }
        }

        impl ::core::ops::AddAssign<usize> for $type<'static> {
            #[inline(always)]
            fn add_assign(&mut self, rhs: usize) {
                **self += rhs;
            }
        }

        impl ::core::ops::AddAssign<isize> for $type<'static> {
            #[inline(always)]
            fn add_assign(&mut self, rhs: isize) {
                **self = (**self as isize + rhs) as usize;
            }
        }

        impl ::core::ops::SubAssign<usize> for $type<'static> {
            #[inline(always)]
            fn sub_assign(&mut self, rhs: usize) {
                **self -= rhs;
            }
        }

        impl ::core::ops::SubAssign<isize> for $type<'static> {
            #[inline(always)]
            fn sub_assign(&mut self, rhs: isize) {
                **self = (**self as isize - rhs) as usize;
            }
        }

        // Bitwise operations
        impl ::core::ops::BitAnd<usize> for $type<'static> {
            type Output = Self;

            #[inline(always)]
            fn bitand(mut self, rhs: usize) -> Self::Output {
                *self &= rhs;
                self
            }
        }

        impl ::core::ops::BitAnd<$type<'static>> for $type<'static> {
            type Output = Self;

            #[inline(always)]
            fn bitand(mut self, rhs: $type) -> Self::Output {
                *self &= *rhs;
                self
            }
        }

        impl ::core::ops::BitOr<usize> for $type<'static> {
            type Output = Self;

            #[inline(always)]
            fn bitor(mut self, rhs: usize) -> Self::Output {
                *self |= rhs;
                self
            }
        }

        impl ::core::ops::BitOr<$type<'static>> for $type<'static> {
            type Output = Self;

            #[inline(always)]
            fn bitor(mut self, rhs: $type) -> Self::Output {
                *self |= *rhs;
                self
            }
        }

        impl ::core::ops::BitXor<usize> for $type<'static> {
            type Output = Self;

            #[inline(always)]
            fn bitxor(mut self, rhs: usize) -> Self::Output {
                *self ^= rhs;
                self
            }
        }

        impl ::core::ops::BitXor<$type<'static>> for $type<'static> {
            type Output = Self;

            #[inline(always)]
            fn bitxor(mut self, rhs: $type) -> Self::Output {
                *self ^= *rhs;
                self
            }
        }

        impl ::core::ops::Not for $type<'static> {
            type Output = Self;

            #[inline(always)]
            fn not(mut self) -> Self::Output {
                *self = !*self;
                self
            }
        }

        impl ::core::ops::Shl<usize> for $type<'static> {
            type Output = Self;

            #[inline(always)]
            fn shl(mut self, rhs: usize) -> Self::Output {
                *self <<= rhs;
                self
            }
        }

        impl ::core::ops::Shr<usize> for $type<'static> {
            type Output = Self;

            #[inline(always)]
            fn shr(mut self, rhs: usize) -> Self::Output {
                *self >>= rhs;
                self
            }
        }

        // Bitwise assignment operations
        impl ::core::ops::BitAndAssign<usize> for $type<'static> {
            #[inline(always)]
            fn bitand_assign(&mut self, rhs: usize) {
                **self &= rhs;
            }
        }

        impl ::core::ops::BitAndAssign<$type<'static>> for $type<'static> {
            #[inline(always)]
            fn bitand_assign(&mut self, rhs: $type) {
                *self &= *rhs;
            }
        }

        impl ::core::ops::BitOrAssign<usize> for $type<'static> {
            #[inline(always)]
            fn bitor_assign(&mut self, rhs: usize) {
                **self |= rhs;
            }
        }

        impl ::core::ops::BitOrAssign<$type<'static>> for $type<'static> {
            #[inline(always)]
            fn bitor_assign(&mut self, rhs: $type) {
                *self |= *rhs;
            }
        }

        impl ::core::ops::BitXorAssign<usize> for $type<'static> {
            #[inline(always)]
            fn bitxor_assign(&mut self, rhs: usize) {
                **self ^= rhs;
            }
        }

        impl ::core::ops::BitXorAssign<$type<'static>> for $type<'static> {
            #[inline(always)]
            fn bitxor_assign(&mut self, rhs: $type) {
                *self ^= *rhs;
            }
        }

        impl ::core::ops::ShlAssign<usize> for $type<'static> {
            #[inline(always)]
            fn shl_assign(&mut self, rhs: usize) {
                **self <<= rhs;
            }
        }

        impl ::core::ops::ShrAssign<usize> for $type<'static> {
            #[inline(always)]
            fn shr_assign(&mut self, rhs: usize) {
                **self >>= rhs;
            }
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn test_const_constructors() {
                const ADDR1: $type = $type::new(0x1234);
                const ADDR2: $type = $type::from(0x1234usize);
                const NULL: $type = $type::null;
                const DEFAULT: $type = <$type as ::core::default::Default>::default();
                const USIZE: usize = usize::from(ADDR1);

                assert_eq!(*ADDR1, 0x1234);
                assert_eq!(ADDR1, ADDR2);
                assert_eq!(USIZE, 0x1234);
                assert_eq!(NULL, DEFAULT);
                assert!(NULL.is_null());
            }

            #[test]
            fn test_const_arithmetic() {
                const ADDR1: $type = $type::new(0x1234);
                const ADDR2: $type = $type::new(0x5678);

                // Test constant addition with operators
                const ADD_USIZE: $type = ADDR1 + 0x100usize;
                const ADD_ISIZE: $type = ADDR1 + 0x100isize;
                const ADD_ADDR: $type = ADDR1 + ADDR2;

                // Test constant subtraction with operators
                const SUB_USIZE: $type = ADDR2 - 0x100usize;
                const SUB_ISIZE: $type = ADDR2 - 0x100isize;
                const SUB_ADDR_DIFF: isize = ADDR2 - ADDR1;

                assert_eq!(*ADD_USIZE, 0x1334);
                assert_eq!(*ADD_ISIZE, 0x1334);
                assert_eq!(*ADD_ADDR, 0x68ac);
                assert_eq!(*SUB_USIZE, 0x5578);
                assert_eq!(*SUB_ISIZE, 0x5578);
                assert_eq!(SUB_ADDR_DIFF, 0x4444);
            }

            #[test]
            fn test_arithmetic_operations() {
                let mut addr = $type::new(0x1000);

                // Test addition
                assert_eq!(addr + 0x100usize, $type::new(0x1100));
                assert_eq!(addr + 0x100isize, $type::new(0x1100));
                assert_eq!(addr + (-0x100isize), $type::new(0xf00));
                assert_eq!(addr + $type::new(0x500), $type::new(0x1500));

                // Test subtraction
                assert_eq!(addr - 0x100usize, $type::new(0xf00));
                assert_eq!(addr - 0x100isize, $type::new(0xf00));
                assert_eq!(addr - (-0x100isize), $type::new(0x1100));

                // Test address difference
                let addr2 = $type::new(0x2000);
                assert_eq!(addr2 - addr, 0x1000isize);
                assert_eq!(addr - addr2, -0x1000isize);

                // Test add assign
                addr += 0x200usize;
                assert_eq!(addr, $type::new(0x1200));

                addr += 0x100isize;
                assert_eq!(addr, $type::new(0x1300));

                addr += (-0x100isize);
                assert_eq!(addr, $type::new(0x1200));

                // Test sub assign
                addr -= 0x100usize;
                assert_eq!(addr, $type::new(0x1100));

                addr -= 0x100isize;
                assert_eq!(addr, $type::new(0x1000));

                addr -= (-0x100isize);
                assert_eq!(addr, $type::new(0x1100));
            }

            #[test]
            fn test_comparisons_and_conversions() {
                let addr1 = $type::new(0x1000);
                let addr2 = $type::new(0x2000);
                let addr3 = $type::new(0x1000);

                // Test equality
                assert_eq!(addr1, addr3);
                assert_ne!(addr1, addr2);
                assert_eq!(addr1, 0x1000usize);
                assert_ne!(addr1, 0x2000usize);

                // Test ordering
                assert!(addr1 < addr2);
                assert!(addr2 > addr1);
                assert!(addr1 <= addr3);
                assert!(addr1 >= addr3);

                // Test conversions
                let usize_val: usize = addr1.into();
                assert_eq!(usize_val, 0x1000);

                let addr_from_usize = $type::from(0x3000usize);
                assert_eq!(*addr_from_usize, 0x3000);

                // Test deref
                assert_eq!(*addr1, 0x1000);

                // Test default
                let default_addr = $type::default();
                assert_eq!(default_addr, $type::null);
                assert!(default_addr.is_null());

                // Test Clone and Copy
                let cloned = addr1.clone();
                let copied = addr1;
                assert_eq!(cloned, addr1);
                assert_eq!(copied, addr1);
            }

            #[test]
            fn test_edge_cases() {
                // Test maximum value
                let max_addr = $type::new(usize::MAX);
                assert_eq!(*max_addr, usize::MAX);

                // Test null address
                let null_addr = $type::null;
                assert!(null_addr.is_null());
                assert_eq!(*null_addr, 0);

                // Test very large addresses
                let large_addr = $type::new(0xFFFF_FFFF_FFFF_0000);
                assert_eq!(*large_addr, 0xFFFF_FFFF_FFFF_0000);

                // Test negative arithmetic that could underflow
                let small_addr = $type::new(0x100);
                let result = small_addr + (-0x200isize);
                // This should wrap around in usize arithmetic
                assert_eq!(*result as isize, (0x100isize - 0x200isize) as usize as isize);

                // Test zero arithmetic
                let zero_addr = $type::new(0);
                assert_eq!(zero_addr + 0usize, zero_addr);
                assert_eq!(zero_addr - 0usize, zero_addr);
                assert_eq!(zero_addr + 0isize, zero_addr);
                assert_eq!(zero_addr - 0isize, zero_addr);

                // Test self arithmetic
                let addr = $type::new(0x1000);
                assert_eq!(addr + $type::new(0), addr);
                assert_eq!(addr - addr, 0isize);
            }

            #[test]
            fn test_display_and_debug() {
                let addr = $type::new(0x1234ABCD);

                // Test Debug formatting with actual string comparison
                let debug_output = format!("{:?}", addr);
                let expected_debug = format!("{}(0x1234abcd)", stringify!($type));
                assert_eq!(debug_output, expected_debug);

                // Test Display formatting
                let display_output = format!("{}", addr);
                let expected_display = format!("{}(0x1234abcd)", stringify!($type));
                assert_eq!(display_output, expected_display);

                // Test null address formatting
                let null_addr = $type::null;
                let null_debug = format!("{:?}", null_addr);
                let expected_null_debug = format!("{}(0x0)", stringify!($type));
                assert_eq!(null_debug, expected_null_debug);

                let null_display = format!("{}", null_addr);
                let expected_null_display = format!("{}(0x0)", stringify!($type));
                assert_eq!(null_display, expected_null_display);

                // Test with maximum value
                let max_addr = $type::new(usize::MAX);
                let max_debug = format!("{:?}", max_addr);
                let expected_max = format!("{}({:#x})", stringify!($type), usize::MAX);
                assert_eq!(max_debug, expected_max);

                // Test different formatting options
                let test_addr = $type::new(0xDEAD_BEEF);
                assert_eq!(format!("{:?}", test_addr), format!("{}(0xdeadbeef)", stringify!($type)));
                assert_eq!(format!("{}", test_addr), format!("{}(0xdeadbeef)", stringify!($type)));

                // Test that formatting works with zero-padded hex
                let small_addr = $type::new(0x42);
                assert_eq!(format!("{:?}", small_addr), format!("{}(0x42)", stringify!($type)));
            }

            #[test]
            fn test_mutable_deref() {
                let mut addr = $type::new(0x1000);

                // Test mutable dereference
                *addr = 0x2000;
                assert_eq!(*addr, 0x2000);
                assert_eq!(*addr, 0x2000);

                // Test that we can modify through deref_mut
                *addr += 0x1000;
                assert_eq!(*addr, 0x3000);
            }

            #[test]
            fn test_bitwise_operations() {
                let addr1 = $type::new(0b11110000);
                let addr2 = $type::new(0b10101010);

                // Test bitwise AND
                let and_result = addr1 & addr2;
                assert_eq!(*and_result, 0b10100000);

                let and_usize = addr1 & 0b11001100usize;
                assert_eq!(*and_usize, 0b11000000);

                // Test bitwise OR
                let or_result = addr1 | addr2;
                assert_eq!(*or_result, 0b11111010);

                let or_usize = addr1 | 0b00001111usize;
                assert_eq!(*or_usize, 0b11111111);

                // Test bitwise XOR
                let xor_result = addr1 ^ addr2;
                assert_eq!(*xor_result, 0b01011010);

                let xor_usize = addr1 ^ 0b11111111usize;
                assert_eq!(*xor_usize, 0b00001111);

                // Test bitwise NOT
                let not_result = !$type::new(0b11110000);
                assert_eq!(*not_result, !0b11110000usize);

                // Test left shift
                let shl_result = $type::new(0b1111) << 4;
                assert_eq!(*shl_result, 0b11110000);

                // Test right shift
                let shr_result = $type::new(0b11110000) >> 4;
                assert_eq!(*shr_result, 0b1111);
            }

            #[test]
            fn test_bitwise_assign_operations() {
                let mut addr = $type::new(0b11110000);

                // Test AND assign
                addr &= 0b10101010usize;
                assert_eq!(*addr, 0b10100000);

                // Test OR assign
                addr |= 0b00001111usize;
                assert_eq!(*addr, 0b10101111);

                // Test XOR assign
                addr ^= 0b11111111usize;
                assert_eq!(*addr, 0b01010000);

                // Test left shift assign
                addr <<= 1;
                assert_eq!(*addr, 0b10100000);

                // Test right shift assign
                addr >>= 2;
                assert_eq!(*addr, 0b00101000);

                // Test with address types
                let mut addr1 = $type::new(0b11110000);
                let addr2 = $type::new(0b10101010);

                addr1 &= addr2;
                assert_eq!(*addr1, 0b10100000);

                addr1 |= $type::new(0b00001111);
                assert_eq!(*addr1, 0b10101111);

                addr1 ^= $type::new(0b11111111);
                assert_eq!(*addr1, 0b01010000);
            }

            #[test]
            fn test_alignment_operations() {
                // Test page alignment (4KB = 0x1000)
                let unaligned = $type::new(0x1234);

                // Test align_down
                let aligned_down = unaligned.align_down(0x1000);
                assert_eq!(*aligned_down, 0x1000);

                // Test align_up
                let aligned_up = unaligned.align_up(0x1000);
                assert_eq!(*aligned_up, 0x2000);

                // Test with already aligned address
                let already_aligned = $type::new(0x2000);
                assert_eq!(already_aligned.align_down(0x1000), already_aligned);
                assert_eq!(already_aligned.align_up(0x1000), already_aligned);

                // Test is_aligned
                assert!(already_aligned.is_aligned(0x1000));
                assert!(!unaligned.is_aligned(0x1000));
                assert!(already_aligned.is_aligned(0x100));
                assert!(already_aligned.is_aligned(0x10));
                assert!(already_aligned.is_aligned(0x1));

                // Test offset_from_alignment
                assert_eq!(unaligned.offset_from_alignment(0x1000), 0x234);
                assert_eq!(already_aligned.offset_from_alignment(0x1000), 0);

                // Test with different alignments
                let addr = $type::new(0x12345678);
                assert_eq!(addr.align_down(0x100), $type::new(0x12345600));
                assert_eq!(addr.align_up(0x100), $type::new(0x12345700));
                assert_eq!(addr.offset_from_alignment(0x100), 0x78);
            }

            #[test]
            fn test_const_bitwise_operations() {
                const ADDR1: $type = $type::new(0b11110000);
                const ADDR2: $type = $type::new(0b10101010);

                // Test const bitwise operations
                const AND_RESULT: $type = $type::new(*ADDR1 & *ADDR2);
                const OR_RESULT: $type = $type::new(*ADDR1 | *ADDR2);
                const XOR_RESULT: $type = $type::new(*ADDR1 ^ *ADDR2);
                const NOT_RESULT: $type = $type::new(!*ADDR1);

                assert_eq!(*AND_RESULT, 0b10100000);
                assert_eq!(*OR_RESULT, 0b11111010);
                assert_eq!(*XOR_RESULT, 0b01011010);
                assert_eq!(*NOT_RESULT, !0b11110000usize);

                // Test const alignment operations
                const UNALIGNED: $type = $type::new(0x1234);
                const ALIGNED_DOWN: $type = UNALIGNED.align_down(0x1000);
                const ALIGNED_UP: $type = UNALIGNED.align_up(0x1000);
                const IS_ALIGNED: bool = ALIGNED_DOWN.is_aligned(0x1000);
                const OFFSET: usize = UNALIGNED.offset_from_alignment(0x1000);

                assert_eq!(*ALIGNED_DOWN, 0x1000);
                assert_eq!(*ALIGNED_UP, 0x2000);
                assert!(IS_ALIGNED);
                assert_eq!(OFFSET, 0x234);
            }

            #[test]
            fn test_page_operations() {
                // Test common page operations
                let page_size = 4096usize; // 4KB pages

                // Test getting page number
                let addr = $type::new(0x12345678);
                let page_number = *addr / page_size;
                let page_base = $type::new(page_number * page_size);

                assert_eq!(page_base, addr.align_down(page_size));

                // Test page boundaries
                let page_start = $type::new(0x1000);
                let page_end = page_start + page_size;

                assert_eq!(*page_end, 0x2000);
                assert!(page_start.is_aligned(page_size));
                assert!(page_end.is_aligned(page_size));

                // Test addresses within page
                let addr_in_page = page_start + 0x123usize;
                assert!(!addr_in_page.is_aligned(page_size));
                assert_eq!(addr_in_page.align_down(page_size), page_start);
                assert_eq!(addr_in_page.align_up(page_size), page_end);
            }

            #[test]
            fn test_addr_non_power_of_two_align() {
                let addr = $type::new(1024);

                // Test align_down with non-power-of-two alignment
                let aligned_down = addr.align_down(100);
                assert_eq!(*aligned_down, 1000);

                // Test align_up with non-power-of-two alignment
                let aligned_up = addr.align_up(100);
                assert_eq!(*aligned_up, 1100);

                // Test is_aligned with non-power-of-two alignment
                assert!(!addr.is_aligned(100));
                assert!(aligned_down.is_aligned(100));
                assert!(aligned_up.is_aligned(100));

                // Test offset_from_alignment with non-power-of-two alignment
                assert_eq!(addr.offset_from_alignment(100), 24);
                assert_eq!(aligned_down.offset_from_alignment(100), 0);
                assert_eq!(aligned_up.offset_from_alignment(100), 0);
            }

            #[test]
            #[cfg(debug_assertions)]
            #[should_panic]
            fn test_align_down_zero_align() {
                let addr = $type::new(0x1234);
                let _ = addr.align_down(0); // Should panic in debug mode
            }

            #[test]
            #[cfg(debug_assertions)]
            #[should_panic]
            fn test_align_up_zero_align() {
                let addr = $type::new(0x1234);
                let _ = addr.align_up(0); // Should panic in debug mode
            }

            #[test]
            #[cfg(debug_assertions)]
            #[should_panic]
            fn test_offset_from_alignment_zero_align() {
                let addr = $type::new(0x1234);
                let _ = addr.offset_from_alignment(0); // Should panic in debug mode
            }

            #[test]
            #[cfg(debug_assertions)]
            #[should_panic]
            fn test_is_aligned_zero_align() {
                let addr = $type::new(0x1234);
                let _ = addr.is_aligned(0); // Should panic in debug modes
            }
        }
    };
}
