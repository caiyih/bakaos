/// Macro to implement range types for address types
macro_rules! impl_range {
    ($range_type:tt, $addr_type:tt, $(#[$doc:meta])*) => {
        $(#[$doc])*
        #[repr(C)]
        #[derive(Clone, Copy, Eq)]
        pub struct $range_type {
            start: $addr_type<'static>,
            end: $addr_type<'static>,
        }

        impl ::core::fmt::Debug for $range_type {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                write!(f, concat!(stringify!($range_type), "({:?}..{:?})"), self.start, self.end)
            }
        }

        impl const ::core::cmp::PartialEq for $range_type {
            #[inline(always)]
            fn eq(&self, other: &Self) -> bool {
                self.start == other.start && self.end == other.end
            }
        }

        impl $range_type {
            /// Creates a new range from start and end addresses.
            ///
            /// # Examples
            /// ```
            /// # use address_v2::{PhysAddr, PhysAddrRange};
            /// let range = PhysAddrRange::new(PhysAddr::new(0x1000), PhysAddr::new(0x2000));
            /// assert_eq!(range.len(), 0x1000);
            /// ```
            #[inline(always)]
            pub const fn new(start: $addr_type<'static>, end: $addr_type<'static>) -> Self {
                // Use deref to access the inner usize value
                debug_assert!(*start <= *end, "Range start must be <= end");
                Self { start, end }
            }

            /// Creates a new range from start and end addresses without checking.
            ///
            /// # Safety
            /// The caller must ensure that start <= end.
            ///
            /// # Examples
            ///
            /// ```
            /// # use address_v2::{PhysAddr, PhysAddrRange};
            /// let range = unsafe { PhysAddrRange::new_unchecked(PhysAddr::new(0x1000), PhysAddr::new(0x2000)) };
            /// assert_eq!(range.len(), 0x1000);
            /// ```
            #[inline(always)]
            pub const unsafe fn new_unchecked(start: $addr_type<'static>, end: $addr_type<'static>) -> Self {
                Self { start, end }
            }

            /// Creates a new range from start address and length.
            ///
            /// # Examples
            /// ```
            /// # use address_v2::{PhysAddr, PhysAddrRange};
            /// let range = PhysAddrRange::from_start_len(PhysAddr::new(0x1000), 0x1000);
            /// assert_eq!(range.start(), PhysAddr::new(0x1000));
            /// assert_eq!(range.end(), PhysAddr::new(0x2000));
            /// ```
            #[inline(always)]
            pub const fn from_start_len(start: $addr_type<'static>, len: usize) -> Self {
                Self::new(start, $addr_type::new(*start + len))
            }

            /// Returns the start address of the range.
            #[inline(always)]
            pub const fn start(&self) -> $addr_type<'static> {
                self.start
            }

            /// Returns the end address of the range (exclusive).
            #[inline(always)]
            pub const fn end(&self) -> $addr_type<'static> {
                self.end
            }

            /// Returns the length of the range in bytes.
            #[inline(always)]
            pub const fn len(&self) -> usize {
                *self.end - *self.start
            }

            /// Checks if the range is empty.
            #[inline(always)]
            pub const fn is_empty(&self) -> bool {
                *self.start == *self.end
            }

            /// Checks if the range contains the given address.
            ///
            /// # Examples
            /// ```
            /// # use address_v2::{PhysAddr, PhysAddrRange};
            /// let range = PhysAddrRange::new(PhysAddr::new(0x1000), PhysAddr::new(0x2000));
            /// assert!(range.contains_addr(PhysAddr::new(0x1500)));
            /// assert!(!range.contains_addr(PhysAddr::new(0x2000))); // end is exclusive
            /// ```
            #[inline(always)]
            pub const fn contains_addr(&self, addr: $addr_type) -> bool {
                *addr >= *self.start && *addr < *self.end
            }

            /// Checks if this range fully contains another range.
            ///
            /// # Examples
            /// ```
            /// # use address_v2::{PhysAddr, PhysAddrRange};
            /// let outer = PhysAddrRange::new(PhysAddr::new(0x1000), PhysAddr::new(0x3000));
            /// let inner = PhysAddrRange::new(PhysAddr::new(0x1500), PhysAddr::new(0x2500));
            /// assert!(outer.contains(inner));
            /// ```
            #[inline(always)]
            pub const fn contains(&self, range: $range_type) -> bool {
                *range.start >= *self.start && *range.end <= *self.end
            }

            /// Checks if this range overlaps with another range.
            ///
            /// # Examples
            /// ```
            /// # use address_v2::{PhysAddr, PhysAddrRange};
            /// let range1 = PhysAddrRange::new(PhysAddr::new(0x1000), PhysAddr::new(0x2000));
            /// let range2 = PhysAddrRange::new(PhysAddr::new(0x1500), PhysAddr::new(0x2500));
            /// assert!(range1.overlaps(range2));
            /// ```
            #[inline(always)]
            pub const fn overlaps(self, other: Self) -> bool {
                *self.start < *other.end && *other.start < *self.end
            }

            /// Checks if this range is adjacent to another range.
            /// Two ranges are adjacent if one ends where the other starts.
            ///
            /// # Examples
            /// ```
            /// # use address_v2::{PhysAddr, PhysAddrRange};
            /// let range1 = PhysAddrRange::new(PhysAddr::new(0x1000), PhysAddr::new(0x2000));
            /// let range2 = PhysAddrRange::new(PhysAddr::new(0x2000), PhysAddr::new(0x3000));
            /// assert!(range1.is_adjacent(range2));
            /// ```
            #[inline(always)]
            pub const fn is_adjacent(self, other: Self) -> bool {
                *self.end == *other.start || *other.end == *self.start
            }

            /// Checks if this range can be merged with another range.
            /// Ranges can be merged if they overlap or are adjacent.
            #[inline(always)]
            pub const fn can_merge(self, other: Self) -> bool {
                self.overlaps(other) || self.is_adjacent(other)
            }

            /// Merges this range with another range if possible.
            /// Returns None if the ranges cannot be merged (not overlapping or adjacent).
            ///
            /// # Examples
            /// ```
            /// # use address_v2::{PhysAddr, PhysAddrRange};
            /// let range1 = PhysAddrRange::new(PhysAddr::new(0x1000), PhysAddr::new(0x2000));
            /// let range2 = PhysAddrRange::new(PhysAddr::new(0x1500), PhysAddr::new(0x2500));
            /// let merged = range1.merge(range2).unwrap();
            /// assert_eq!(merged.start(), PhysAddr::new(0x1000));
            /// assert_eq!(merged.end(), PhysAddr::new(0x2500));
            /// ```
            pub const fn merge(self, other: Self) -> Option<Self> {
                if self.can_merge(other) {
                    let start = if *self.start < *other.start { self.start } else { other.start };
                    let end = if *self.end > *other.end { self.end } else { other.end };
                    Some(Self::new(start, end))
                } else {
                    None
                }
            }

            /// Returns the intersection of this range with another range.
            /// Returns None if the ranges don't overlap.
            ///
            /// # Examples
            /// ```
            /// # use address_v2::{PhysAddr, PhysAddrRange};
            /// let range1 = PhysAddrRange::new(PhysAddr::new(0x1000), PhysAddr::new(0x2000));
            /// let range2 = PhysAddrRange::new(PhysAddr::new(0x1500), PhysAddr::new(0x2500));
            /// let intersection = range1.intersection(range2).unwrap();
            /// assert_eq!(intersection.start(), PhysAddr::new(0x1500));
            /// assert_eq!(intersection.end(), PhysAddr::new(0x2000));
            /// ```
            pub const fn intersection(self, other: Self) -> Option<Self> {
                if self.overlaps(other) {
                    let start = if *self.start > *other.start { self.start } else { other.start };
                    let end = if *self.end < *other.end { self.end } else { other.end };
                    Some(Self::new(start, end))
                } else {
                    None
                }
            }

            /// Aligns the range to the given alignment.
            /// The start is aligned down and the end is aligned up.
            ///
            /// # Examples
            /// ```
            /// # use address_v2::{PhysAddr, PhysAddrRange};
            /// let range = PhysAddrRange::new(PhysAddr::new(0x1234), PhysAddr::new(0x2345));
            /// let aligned = range.align_to(0x1000);
            /// assert_eq!(aligned.start(), PhysAddr::new(0x1000));
            /// assert_eq!(aligned.end(), PhysAddr::new(0x3000));
            /// ```
            #[inline(always)]
            pub const fn align_to(self, align: usize) -> Self {
                Self::new(
                    self.start.align_down(align),
                    self.end.align_up(align)
                )
            }

            /// Returns an iterator over addresses in this range with the given step size.
            /// Returns `None` if `step == 0` or if the range length is not a multiple of `step`.
            ///
            /// # Examples
            /// ```
            /// # use address_v2::{PhysAddr, PhysAddrRange};
            /// let range = PhysAddrRange::new(PhysAddr::new(0x1000), PhysAddr::new(0x1010));
            /// let addresses: Vec<_> = range.iter_step(4).unwrap().collect();
            /// assert_eq!(addresses.len(), 4);
            /// assert_eq!(addresses[0], PhysAddr::new(0x1000));
            /// assert_eq!(addresses[3], PhysAddr::new(0x100c));
            /// ```
            #[inline(always)]
            pub const fn iter_step(self, step: usize) -> Option<RangeIterator> {
                RangeIterator::new(self, step)
            }

            /// Returns an iterator over addresses in this range with step size 1.
            #[inline(always)]
            pub const fn iter(self) -> Option<RangeIterator> {
                self.iter_step(1)
            }

            /// Returns an iterator over page-aligned addresses in this range.
            /// Uses 4KB (0x1000) as the default page size.
            #[inline(always)]
            pub const fn iter_pages(self) -> Option<RangeIterator> {
                self.iter_step(0x1000)
            }

            /// Returns an iterator over addresses in this range with custom page size.
            #[inline(always)]
            pub const fn iter_pages_sized(self, page_size: usize) -> Option<RangeIterator> {
                self.iter_step(page_size)
            }
        }

        impl ::core::fmt::Display for $range_type {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                write!(f, "{}({:?}..{:?})", stringify!($range_type), self.start, self.end)
            }
        }

        impl const ::core::default::Default for $range_type {
            #[inline(always)]
            fn default() -> Self {
                unsafe { Self::new_unchecked($addr_type::null, $addr_type::null) }
            }
        }

        /// Iterator over addresses in a range
        pub struct RangeIterator {
            current: $addr_type<'static>,
            end: $addr_type<'static>,
            step: usize,
        }

        impl RangeIterator
        {
            #[inline(always)]
            pub const fn new(range: $range_type, step: usize) -> Option<Self> {
                let len: usize = *range.end - *range.start;

                if step != 0 && len.is_multiple_of(step) {
                    Some(Self {
                        current: range.start,
                        end: range.end,
                        step,
                    })
                } else {
                    None
                }
            }

            /// Creates a new iterator without checking the range validity.
            ///
            /// # Safety
            ///
            /// The caller must ensure that the range is valid and the step is non-zero and divides the range length.
            #[inline]
            pub const unsafe fn new_unchecked(range: $range_type, step: usize) -> Self {
                Self {
                    current: range.start,
                    end: range.end,
                    step,
                }
            }
        }

        impl ::core::iter::Iterator for RangeIterator
        {
            type Item = $addr_type<'static>;

            fn next(&mut self) -> Option<Self::Item> {
                let new_current = *self.current + self.step;

                // Ensure this range doen't overflow and stays within bounds
                if new_current > *self.end {
                    None
                } else {
                    let result = self.current;
                    self.current = $addr_type::new(new_current);
                    Some(result)
                }
            }
        }

        impl ::core::iter::ExactSizeIterator for RangeIterator
        {
            fn len(&self) -> usize {
                let current_usize: usize = *self.current;
                let end_usize: usize = *self.end;

                debug_assert!(current_usize <= end_usize);

                (end_usize - current_usize) / self.step
            }
        }

        #[cfg(test)]
        mod range_tests {
            use super::*;

            #[test]
            fn test_range_creation() {
                let start = $addr_type::new(0x1000);
                let end = $addr_type::new(0x2000);
                let range = $range_type::new(start, end);

                assert_eq!(range.start(), start);
                assert_eq!(range.end(), end);
                assert_eq!(range.len(), 0x1000);
                assert!(!range.is_empty());
            }

            #[test]
            fn test_equality() {
                const RANGE1: $range_type = $range_type::new($addr_type::new(0x1000), $addr_type::new(0x2000));
                const RANGE2: $range_type = $range_type::new($addr_type::new(0x1000), $addr_type::new(0x2000));
                const RANGE3: $range_type = $range_type::new($addr_type::new(0x1000), $addr_type::new(0x3000));
                const RANGE4: $range_type = $range_type::new($addr_type::new(0x0000), $addr_type::new(0x2000));

                assert_eq!(RANGE1, RANGE2);
                assert_ne!(RANGE1, RANGE3);
                assert_ne!(RANGE1, RANGE4);
            }

            #[test]
            fn test_range_from_start_len() {
                let start = $addr_type::new(0x1000);
                let range = $range_type::from_start_len(start, 0x1000);

                assert_eq!(range.start(), start);
                assert_eq!(range.end(), $addr_type::new(0x2000));
                assert_eq!(range.len(), 0x1000);
            }

            #[test]
            fn test_empty_range() {
                let addr = $addr_type::new(0x1000);
                let range = $range_type::new(addr, addr);

                assert!(range.is_empty());
                assert_eq!(range.len(), 0);
            }

            #[test]
            fn test_addr_contains() {
                let range = $range_type::new($addr_type::new(0x1000), $addr_type::new(0x2000));

                assert!(range.contains_addr($addr_type::new(0x1000))); // start inclusive
                assert!(range.contains_addr($addr_type::new(0x1500)));
                assert!(range.contains_addr($addr_type::new(0x1fff)));
                assert!(!range.contains_addr($addr_type::new(0x2000))); // end exclusive
                assert!(!range.contains_addr($addr_type::new(0x500)));
            }

            #[test]
            fn test_range_contains() {
                let range1 = $range_type::new($addr_type::new(0x1000), $addr_type::new(0x2000));
                let range2 = $range_type::new($addr_type::new(0x1200), $addr_type::new(0x1F00));
                let range3 = $range_type::new($addr_type::new(0x0F00), $addr_type::new(0x1500));
                let range4 = $range_type::new($addr_type::new(0x0E00), $addr_type::new(0x1100));

                assert!(range1.contains(range2));
                assert!(!range1.contains(range3));
                assert!(!range1.contains(range4));
            }


            #[test]
            fn test_range_overlaps() {
                let range1 = $range_type::new($addr_type::new(0x1000), $addr_type::new(0x2000));
                let range2 = $range_type::new($addr_type::new(0x1500), $addr_type::new(0x2500));
                let range3 = $range_type::new($addr_type::new(0x3000), $addr_type::new(0x4000));

                assert!(range1.overlaps(range2));
                assert!(range2.overlaps(range1));
                assert!(!range1.overlaps(range3));
                assert!(!range3.overlaps(range1));
            }

            #[test]
            fn test_range_adjacent() {
                let range1 = $range_type::new($addr_type::new(0x1000), $addr_type::new(0x2000));
                let range2 = $range_type::new($addr_type::new(0x2000), $addr_type::new(0x3000));
                let range3 = $range_type::new($addr_type::new(0x4000), $addr_type::new(0x5000));

                assert!(range1.is_adjacent(range2));
                assert!(range2.is_adjacent(range1));
                assert!(!range1.is_adjacent(range3));
            }

            #[test]
            fn test_range_merge() {
                let range1 = $range_type::new($addr_type::new(0x1000), $addr_type::new(0x2000));
                let range2 = $range_type::new($addr_type::new(0x1500), $addr_type::new(0x2500));
                let range3 = $range_type::new($addr_type::new(0x4000), $addr_type::new(0x5000));

                // Test overlapping merge
                let merged = range1.merge(range2).unwrap();
                assert_eq!(merged.start(), $addr_type::new(0x1000));
                assert_eq!(merged.end(), $addr_type::new(0x2500));

                // Test adjacent merge
                let range4 = $range_type::new($addr_type::new(0x2000), $addr_type::new(0x3000));
                let merged_adj = range1.merge(range4).unwrap();
                assert_eq!(merged_adj.start(), $addr_type::new(0x1000));
                assert_eq!(merged_adj.end(), $addr_type::new(0x3000));

                // Test non-mergeable
                assert!(range1.merge(range3).is_none());
            }

            #[test]
            fn test_range_intersection() {
                let range1 = $range_type::new($addr_type::new(0x1000), $addr_type::new(0x2000));
                let range2 = $range_type::new($addr_type::new(0x1500), $addr_type::new(0x2500));
                let range3 = $range_type::new($addr_type::new(0x3000), $addr_type::new(0x4000));

                // Test overlapping intersection
                let intersection = range1.intersection(range2).unwrap();
                assert_eq!(intersection.start(), $addr_type::new(0x1500));
                assert_eq!(intersection.end(), $addr_type::new(0x2000));

                // Test non-overlapping
                assert!(range1.intersection(range3).is_none());
            }

            #[test]
            fn test_range_alignment() {
                let range = $range_type::new($addr_type::new(0x1234), $addr_type::new(0x2345));
                let aligned = range.align_to(0x1000);

                assert_eq!(aligned.start(), $addr_type::new(0x1000));
                assert_eq!(aligned.end(), $addr_type::new(0x3000));
                assert!(aligned.start().is_aligned(0x1000));
                assert!(aligned.end().is_aligned(0x1000));
            }

            #[test]
            fn test_range_iterator() {
                let range = $range_type::new($addr_type::new(0x1000), $addr_type::new(0x1010));
                let addresses: Vec<_> = range.iter_step(4).unwrap().collect();

                assert_eq!(addresses.len(), 4);
                assert_eq!(addresses[0], $addr_type::new(0x1000));
                assert_eq!(addresses[1], $addr_type::new(0x1004));
                assert_eq!(addresses[2], $addr_type::new(0x1008));
                assert_eq!(addresses[3], $addr_type::new(0x100c));

                let iter = range.iter().unwrap();
                assert_eq!(iter.len(), 16);
                assert_eq!(iter.step, 1);
            }

            #[test]
            fn test_range_page_iterator() {
                let range = $range_type::new($addr_type::new(0x1000), $addr_type::new(0x4000));
                let pages: Vec<_> = range.iter_pages().unwrap().collect();

                assert_eq!(pages.len(), 3);
                assert_eq!(pages[0], $addr_type::new(0x1000));
                assert_eq!(pages[1], $addr_type::new(0x2000));
                assert_eq!(pages[2], $addr_type::new(0x3000));
            }

            #[test]
            fn test_iterator_length_less_than_step() {
                let range = $range_type::new($addr_type::new(0x1000), $addr_type::new(0x2000));

                assert!(range.iter_step(0x2000).is_none()); // length not multiple of step

                let iter = unsafe { RangeIterator::new_unchecked(range, 0x2000) };
                assert_eq!(iter.len(), 0); // no steps fit
            }

            #[test]
            fn test_range_display() {
                let range = $range_type::new($addr_type::new(0x1000), $addr_type::new(0x2000));
                let display_str = format!("{}", range);
                let expected = format!("{}({}(0x1000)..{}(0x2000))",
                    stringify!($range_type),
                    stringify!($addr_type),
                    stringify!($addr_type));
                assert_eq!(display_str, expected);
            }

            #[test]
            fn test_range_default() {
                let range = $range_type::default();
                assert!(range.is_empty());
                assert_eq!(range.start(), $addr_type::null);
                assert_eq!(range.end(), $addr_type::null);
            }

            #[test]
            fn test_range_debug_format() {
                let range = $range_type::new($addr_type::new(0x1000), $addr_type::new(0x2000));
                let debug_str = format!("{:?}", range);
                let expected = format!("{}({:?}..{:?})",
                    stringify!($range_type),
                    $addr_type::new(0x1000),
                    $addr_type::new(0x2000));
                assert_eq!(debug_str, expected);
            }
        }
    };
}
