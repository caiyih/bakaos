use core::ops::Range;

use crate::*;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageNumRange<T>
where
    T: IPageNum,
{
    start: T,
    end: T,
}

impl<T> PageNumRange<T>
where
    T: IPageNum,
{
    pub fn new(range: Range<usize>) -> Self {
        PageNumRange {
            start: T::from_usize(range.start),
            end: T::from_usize(range.end),
        }
    }

    pub fn from(range: Range<T>) -> Self {
        PageNumRange {
            start: range.start,
            end: range.end,
        }
    }

    pub fn from_start_count(start: T, count: usize) -> Self {
        PageNumRange {
            start,
            end: T::from_usize(start.as_usize() + count),
        }
    }

    pub fn from_start_end(start: T, end: T) -> Self {
        debug_assert!(start <= end);
        PageNumRange { start, end }
    }

    pub fn form_single(num: T) -> Self {
        PageNumRange {
            start: num,
            end: T::from_usize(num.as_usize() + 1),
        }
    }

    pub fn start(&self) -> T {
        self.start
    }

    pub fn end(&self) -> T {
        self.end
    }

    pub fn page_count(&self) -> usize {
        let count = self.end.diff_page_count(self.start);

        debug_assert!(count >= 0);

        count as usize
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn contains(&self, num: T) -> bool {
        self.start <= num && num < self.end
    }

    pub fn contains_range(&self, other: &PageNumRange<T>) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    pub fn contained_by(&self, other: &PageNumRange<T>) -> bool {
        other.contains_range(self)
    }

    pub fn iter(&self) -> PageNumRangeIter<T> {
        PageNumRangeIter {
            range: *self,
            current: self.start,
        }
    }
}

pub struct PageNumRangeIter<T>
where
    T: IPageNum,
{
    range: PageNumRange<T>,
    current: T,
}

impl<T> Iterator for PageNumRangeIter<T>
where
    T: IPageNum,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match Ord::cmp(&self.current, &self.range.end) {
            core::cmp::Ordering::Less => {
                let current = self.current;
                self.current.step();
                Some(current)
            }
            _ => None,
        }
    }
}
