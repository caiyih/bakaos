use core::{
    fmt::Display,
    ops::{Add, AddAssign, Sub, SubAssign},
};

use crate::*;

pub trait IPageNumBase: Copy + Clone + PartialEq + PartialOrd + Eq + Ord {
    fn from_usize(value: usize) -> Self;

    fn as_usize(self) -> usize;
}

pub trait IPageNumOps:
    IPageNumBase
    + Add<Self>
    + Sub<Self>
    + AddAssign<Self>
    + SubAssign<Self>
    + Add<usize>
    + Sub<usize>
    + AddAssign<usize>
    + SubAssign<usize>
{
}

#[macro_export]
macro_rules! impl_page_num_ops {
    ($type:ty) => {
        impl IPageNumOps for $type {}

        // 与 usize 的运算符实现
        impl core::ops::Add<usize> for $type {
            type Output = Self;
            fn add(self, rhs: usize) -> Self::Output {
                Self::from_usize(self.as_usize() + rhs)
            }
        }

        impl core::ops::Sub<usize> for $type {
            type Output = Self;
            fn sub(self, rhs: usize) -> Self::Output {
                Self::from_usize(self.as_usize() - rhs)
            }
        }

        impl core::ops::AddAssign<usize> for $type {
            fn add_assign(&mut self, rhs: usize) {
                *self = Self::from_usize(self.as_usize() + rhs);
            }
        }

        impl core::ops::SubAssign<usize> for $type {
            fn sub_assign(&mut self, rhs: usize) {
                *self = Self::from_usize(self.as_usize() - rhs);
            }
        }

        // 相同类型之间的运算符实现
        impl core::ops::Add for $type {
            type Output = Self;
            fn add(self, rhs: Self) -> Self::Output {
                Self::from_usize(self.as_usize() + rhs.as_usize())
            }
        }

        impl core::ops::Sub for $type {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self::Output {
                Self::from_usize(self.as_usize() - rhs.as_usize())
            }
        }

        impl core::ops::AddAssign for $type {
            fn add_assign(&mut self, rhs: Self) {
                *self = Self::from_usize(self.as_usize() + rhs.as_usize());
            }
        }

        impl core::ops::SubAssign for $type {
            fn sub_assign(&mut self, rhs: Self) {
                *self = Self::from_usize(self.as_usize() - rhs.as_usize());
            }
        }
    };
}

pub trait IPageNum: IPageNumBase + IPageNumOps + Display {
    fn step(&mut self) {
        self.step_by(1);
    }

    fn step_by(&mut self, offset: usize) {
        *self += offset;
    }

    fn step_back(&mut self) {
        self.step_back_by(1);
    }

    fn step_back_by(&mut self, offset: usize) {
        *self -= offset;
    }

    fn from_addr_floor<T: IAddress>(addr: T) -> Self {
        Self::from_usize(addr.align_down(constants::PAGE_SIZE).as_usize() / constants::PAGE_SIZE)
    }

    fn from_addr_ceil<T: IAddress>(addr: T) -> Self {
        Self::from_usize(addr.align_up(constants::PAGE_SIZE).as_usize() / constants::PAGE_SIZE)
    }

    fn start_addr<T: IAddress>(self) -> T {
        T::from_usize(self.as_usize() * constants::PAGE_SIZE)
    }

    fn end_addr<T: IAddress>(self) -> T {
        T::from_usize((self.as_usize() + 1) * constants::PAGE_SIZE)
    }

    fn at_offset_of_start<T: IAddress>(self, offset: usize) -> T {
        T::from_usize(self.as_usize() * constants::PAGE_SIZE + offset)
    }

    fn at_offset_of_end<T: IAlignableAddress>(self, offset: usize) -> T {
        T::from_usize((self.as_usize() + 1) * constants::PAGE_SIZE - offset)
    }

    fn start_offset_of_addr<T: IAddress>(self, addr: T) -> isize {
        addr.diff(self.start_addr())
    }

    fn end_offset_of_addr<T: IAddress>(self, addr: T) -> isize {
        addr.diff(self.end_addr())
    }

    fn diff_page_count(self, other: Self) -> isize {
        (self.as_usize() as i64 - other.as_usize() as i64) as isize
    }

    fn addr_range<T: IAddress>(self) -> AddressRange<T> {
        AddressRange::from_start_end(self.start_addr(), self.end_addr())
    }
}

#[macro_export]
macro_rules! impl_IPageNum {
    ($type:ty) => {
        impl IPageNumBase for $type {
            #[inline(always)]
            fn from_usize(value: usize) -> Self {
                Self(value)
            }

            #[inline(always)]
            fn as_usize(self) -> usize {
                self.0
            }
        }

        impl_page_num_ops!($type);

        impl IPageNum for $type {}

        impl_usize_display!($type);
    };
}
