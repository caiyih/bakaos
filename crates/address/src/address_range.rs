use core::ops::Range;

use crate::*;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AddressRange<T>
where
    T: IAddress,
{
    start: T,
    end: T,
}

impl<T> AddressRange<T>
where
    T: IAddress,
{
    pub fn from_start_len(start: T, len: usize) -> Self {
        AddressRange {
            start,
            end: T::from_usize(start.as_usize() + len),
        }
    }

    pub fn from_start_end(start: T, end: T) -> Self {
        debug_assert!(start <= end);
        AddressRange { start, end }
    }

    pub fn new(range: Range<T>) -> Self {
        AddressRange {
            start: range.start,
            end: range.end,
        }
    }

    #[inline(always)]
    pub fn start(&self) -> T {
        self.start
    }

    #[inline(always)]
    pub fn end(&self) -> T {
        self.end
    }

    pub fn start_page(&self) -> usize {
        self.start.page_down().as_usize()
    }

    pub fn end_page(&self) -> usize {
        self.end.page_down().as_usize()
    }

    pub fn len(&self) -> usize {
        let diff = self.end.diff(self.start);

        debug_assert!(diff >= 0);

        diff as usize
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn contains(&self, addr: T) -> bool {
        self.start <= addr && addr < self.end
    }

    pub fn contains_range(&self, other: &AddressRange<T>) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    pub fn contained_by(&self, other: &AddressRange<T>) -> bool {
        other.contains_range(self)
    }

    pub fn intersects(&self, other: &AddressRange<T>) -> bool {
        self.start < other.end && other.start < self.end
    }

    pub fn intersection(&self, other: &AddressRange<T>) -> Option<AddressRange<T>> {
        if self.intersects(other) {
            Some(AddressRange {
                start: core::cmp::max(self.start, other.start),
                end: core::cmp::min(self.end, other.end),
            })
        } else {
            None
        }
    }

    pub fn union(&self, other: &AddressRange<T>) -> AddressRange<T> {
        AddressRange {
            start: core::cmp::min(self.start, other.start),
            end: core::cmp::max(self.end, other.end),
        }
    }

    pub fn off_by(&self, offset: isize) -> AddressRange<T> {
        AddressRange {
            start: self.start.off_by(offset),
            end: self.end.off_by(offset),
        }
    }

    pub fn iter(&self) -> AddressRangeIter<T> {
        AddressRangeIter {
            range: self.clone(),
            current: self.start,
        }
    }
}

impl<T> IntoIterator for AddressRange<T>
where
    T: IAddress,
{
    type Item = T;

    type IntoIter = AddressRangeIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[macro_export]
macro_rules! impl_range_display {
    ($type:ty) => {
        impl core::fmt::Display for $type {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(
                    f,
                    "{}({:#x}..{:#x})",
                    stringify!($type),
                    abstractions::IUsizeAlias::as_usize(&self.start()),
                    abstractions::IUsizeAlias::as_usize(&self.end())
                )
            }
        }
    };
}

pub struct AddressRangeIter<T>
where
    T: IAddress,
{
    range: AddressRange<T>,
    current: T,
}

impl<T> Iterator for AddressRangeIter<T>
where
    T: IAddress,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match Ord::cmp(&self.current, &self.range.end) {
            core::cmp::Ordering::Less => {
                let current = self.current;
                self.current.step_by(1);
                Some(current)
            }
            _ => None,
        }
    }
}
