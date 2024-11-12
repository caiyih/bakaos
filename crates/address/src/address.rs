use core::{
    fmt::Display,
    mem::size_of,
    ops::{
        Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Sub,
        SubAssign,
    },
};

pub trait IAddressBase: Copy + Clone + PartialEq + PartialOrd + Eq + Ord {
    fn as_usize(self) -> usize;

    fn from_usize(value: usize) -> Self;
}

pub trait IAddressOps:
    IAddressBase
    + Add<usize>
    + Add<Self>
    + Sub<usize>
    + Sub<Self>
    + AddAssign<usize>
    + AddAssign<Self>
    + SubAssign<usize>
    + SubAssign<Self>
    + BitAnd<usize>
    + BitAnd<Self>
    + BitOr<usize>
    + BitOr<Self>
    + BitXor<usize>
    + BitXor<Self>
    + BitAndAssign<usize>
    + BitAndAssign<Self>
    + BitOrAssign<usize>
    + BitOrAssign<Self>
    + BitXorAssign<usize>
    + BitXorAssign<Self>
{
}

#[macro_export]
macro_rules! impl_address_ops {
    ($type:ty) => {
        impl IAddressOps for $type {}

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

        impl core::ops::BitAnd for $type {
            type Output = Self;
            fn bitand(self, rhs: Self) -> Self::Output {
                Self::from_usize(self.as_usize() & rhs.as_usize())
            }
        }

        impl core::ops::BitOr for $type {
            type Output = Self;
            fn bitor(self, rhs: Self) -> Self::Output {
                Self::from_usize(self.as_usize() | rhs.as_usize())
            }
        }

        impl core::ops::BitXor for $type {
            type Output = Self;
            fn bitxor(self, rhs: Self) -> Self::Output {
                Self::from_usize(self.as_usize() ^ rhs.as_usize())
            }
        }

        impl core::ops::BitAndAssign for $type {
            fn bitand_assign(&mut self, rhs: Self) {
                *self = Self::from_usize(self.as_usize() & rhs.as_usize());
            }
        }

        impl core::ops::BitOrAssign for $type {
            fn bitor_assign(&mut self, rhs: Self) {
                *self = Self::from_usize(self.as_usize() | rhs.as_usize());
            }
        }

        impl core::ops::BitXorAssign for $type {
            fn bitxor_assign(&mut self, rhs: Self) {
                *self = Self::from_usize(self.as_usize() ^ rhs.as_usize());
            }
        }

        impl core::ops::BitAnd<usize> for $type {
            type Output = Self;
            fn bitand(self, rhs: usize) -> Self::Output {
                Self::from_usize(self.as_usize() & rhs)
            }
        }

        impl core::ops::BitOr<usize> for $type {
            type Output = Self;
            fn bitor(self, rhs: usize) -> Self::Output {
                Self::from_usize(self.as_usize() | rhs)
            }
        }

        impl core::ops::BitXor<usize> for $type {
            type Output = Self;
            fn bitxor(self, rhs: usize) -> Self::Output {
                Self::from_usize(self.as_usize() ^ rhs)
            }
        }

        impl core::ops::BitAndAssign<usize> for $type {
            fn bitand_assign(&mut self, rhs: usize) {
                *self = Self::from_usize(self.as_usize() & rhs);
            }
        }

        impl core::ops::BitOrAssign<usize> for $type {
            fn bitor_assign(&mut self, rhs: usize) {
                *self = Self::from_usize(self.as_usize() | rhs);
            }
        }

        impl core::ops::BitXorAssign<usize> for $type {
            fn bitxor_assign(&mut self, rhs: usize) {
                *self = Self::from_usize(self.as_usize() ^ rhs);
            }
        }
    };
}

pub trait IAlignableAddress: IAddressBase {
    fn is_aligned(self, align: usize) -> bool {
        self.as_usize() % align == 0
    }

    fn is_page_aligned(self) -> bool {
        self.is_aligned(constants::PAGE_SIZE)
    }

    fn align_up(self, align: usize) -> Self {
        debug_assert!(align.is_power_of_two());

        let mask = align - 1;
        let aligned = (self.as_usize() + mask) & !mask;
        Self::from_usize(aligned)
    }

    fn align_down(self, align: usize) -> Self {
        debug_assert!(align.is_power_of_two());

        let mask = align - 1;
        let aligned = self.as_usize() & !mask;
        Self::from_usize(aligned)
    }

    fn align_page_up(self) -> Self {
        self.align_up(constants::PAGE_SIZE)
    }

    fn align_page_down(self) -> Self {
        self.align_down(constants::PAGE_SIZE)
    }

    fn page_down(self) -> Self {
        self.align_down(constants::PAGE_SIZE)
    }

    fn page_up(self) -> Self {
        self.align_up(constants::PAGE_SIZE)
    }
}

pub trait IAddress: IAddressBase + IAlignableAddress + IAddressOps + Display {
    fn add_n<T>(self, n: usize) -> Self {
        self.add_by(size_of::<T>() * n)
    }

    fn add<T>(self) -> Self {
        self.add_by(size_of::<T>())
    }

    fn minus_n<T>(self, n: usize) -> Self {
        self.minus_by(size_of::<T>() * n)
    }

    fn minus<T>(self) -> Self {
        self.minus_by(size_of::<T>())
    }

    fn minus_by(self, offset: usize) -> Self {
        Self::from_usize(self.as_usize() - offset)
    }

    fn add_by(self, offset: usize) -> Self {
        Self::from_usize(self.as_usize() + offset)
    }

    fn off_by(self, offset: isize) -> Self {
        Self::from_usize((self.as_usize() as isize + offset) as usize)
    }

    fn in_page_offset(self) -> usize {
        self.as_usize() % constants::PAGE_SIZE
    }

    fn diff(self, other: Self) -> isize {
        (self.as_usize() as i64 - other.as_usize() as i64) as isize
    }

    fn step_back_n<T>(&mut self, n: usize) {
        self.step_back_by(size_of::<T>() * n);
    }

    fn step_back<T>(&mut self) {
        self.step_back_by(size_of::<T>());
    }

    fn step_back_by(&mut self, offset: usize) {
        *self = self.minus_by(offset);
    }

    fn step_n<T>(&mut self, n: usize) {
        self.step_by(size_of::<T>() * n);
    }

    fn step<T>(&mut self) {
        self.step_by(size_of::<T>());
    }

    fn step_by(&mut self, offset: usize) {
        *self = self.add_by(offset);
    }
}

#[macro_export]
macro_rules! impl_IAddress {
    ($type:ty) => {
        impl IAddressBase for $type {
            #[inline(always)]
            fn as_usize(self) -> usize {
                self.0
            }

            #[inline(always)]
            fn from_usize(value: usize) -> Self {
                Self(value)
            }
        }

        impl IAlignableAddress for $type {}

        impl_usize_display!($type);
        impl_address_ops!($type);

        impl IAddress for $type {}
    };
}

#[macro_export]
macro_rules! impl_usize_display {
    ($type:ty) => {
        impl core::fmt::Display for $type {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}({:#x})", stringify!($type), self.as_usize())
            }
        }
    };
}

// #[test]
// fn test_virtual_address_identity_mapping() {
//     let va = VirtualAddress::from_usize(0x1000);
//     let pa = va.identity_mapped();
//     assert_eq!(pa.as_usize(), 0x1000);
// }

// #[test]
// fn test_physical_address_identity_mapping() {
//     let pa = PhysicalAddress::from_usize(0x2000);
//     let va = pa.identity_mapped();
//     assert_eq!(va.as_usize(), 0x2000);
// }

// #[test]
// fn test_address_range_contains_range() {
//     let range1 = AddressRange::from_start_end(
//         VirtualAddress::from_usize(0x1000),
//         VirtualAddress::from_usize(0x2000),
//     );
//     let range2 = AddressRange::from_start_end(
//         VirtualAddress::from_usize(0x1500),
//         VirtualAddress::from_usize(0x1800),
//     );
//     assert!(range1.contains_range(&range2));
//     assert!(range2.contained_by(&range1));
// }

// #[test]
// fn test_address_range_intersects() {
//     let range1 = AddressRange::from_start_end(
//         VirtualAddress::from_usize(0x1000),
//         VirtualAddress::from_usize(0x2000),
//     );
//     let range2 = AddressRange::from_start_end(
//         VirtualAddress::from_usize(0x1800),
//         VirtualAddress::from_usize(0x2800),
//     );
//     assert!(range1.intersects(&range2));
//     let intersection = range1.intersection(&range2).unwrap();
//     assert_eq!(intersection.start().as_usize(), 0x1800);
//     assert_eq!(intersection.end().as_usize(), 0x2000);
// }

// #[test]
// fn test_address_range_union() {
//     let range1 = AddressRange::from_start_end(
//         VirtualAddress::from_usize(0x1000),
//         VirtualAddress::from_usize(0x2000),
//     );
//     let range2 = AddressRange::from_start_end(
//         VirtualAddress::from_usize(0x1800),
//         VirtualAddress::from_usize(0x2800),
//     );
//     let union = range1.union(&range2);
//     assert_eq!(union.start().as_usize(), 0x1000);
//     assert_eq!(union.end().as_usize(), 0x2800);
// }

// #[test]
// fn test_page_num_range_page_count() {
//     let range = PageNumRange::from_start_count(VirtualPageNum::from_usize(5), 10);
//     assert_eq!(range.page_count(), 10);
// }

// #[test]
// fn test_virtual_page_num_identity_mapping() {
//     let vpn = VirtualPageNum::from_usize(42);
//     let ppn = vpn.identity_mapped();
//     assert_eq!(ppn.as_usize(), 42);
// }

// #[test]
// fn test_physical_page_num_identity_mapping() {
//     let ppn = PhysicalPageNum::from_usize(100);
//     let vpn = ppn.identity_mapped();
//     assert_eq!(vpn.as_usize(), 100);
// }

// #[test]
// fn test_address_off_by() {
//     let addr = VirtualAddress::from_usize(0x1000);
//     let addr_offset = addr.off_by(0x200);
//     assert_eq!(addr_offset.as_usize(), 0x1200);
//     let addr_negative_offset = addr.off_by(-0x200);
//     assert_eq!(addr_negative_offset.as_usize(), 0x0E00);
// }

// #[test]
// fn test_virtual_address_as_ptr() {
//     let addr = VirtualAddress::from_usize(0x1000);
//     let ptr = addr.as_ptr::<u32>();
//     assert_eq!(ptr as usize, 0x1000);
// }

// #[test]
// fn test_virtual_address_as_mut_ptr() {
//     let addr = VirtualAddress::from_usize(0x2000);
//     let ptr = addr.as_mut_ptr::<u64>();
//     assert_eq!(ptr as usize, 0x2000);
// }

// #[test]
// fn test_page_num_from_addr_floor_ceil() {
//     let addr = VirtualAddress::from_usize(0x1234);
//     let vpn_floor = VirtualPageNum::from_addr_floor(addr);
//     let vpn_ceil = VirtualPageNum::from_addr_ceil(addr);
//     assert_eq!(vpn_floor.as_usize(), 0x1);
//     assert_eq!(vpn_ceil.as_usize(), 0x2);
// }

// #[test]
// fn test_physical_address_alignment() {
//     let addr = PhysicalAddress::from_usize(0x1234);
//     assert!(!addr.is_aligned(0x1000));
//     let aligned_up = addr.align_up(0x1000);
//     assert_eq!(aligned_up.as_usize(), 0x2000);
//     let aligned_down = addr.align_down(0x1000);
//     assert_eq!(aligned_down.as_usize(), 0x1000);
// }

// #[test]
// fn test_address_range_is_empty() {
//     let range = AddressRange::from_start_end(
//         VirtualAddress::from_usize(0x1000),
//         VirtualAddress::from_usize(0x1000),
//     );
//     assert!(range.is_empty());
// }

// #[test]
// fn test_page_num_range_contains_range() {
//     let range1 = PageNumRange::from_start_end(
//         VirtualPageNum::from_usize(10),
//         VirtualPageNum::from_usize(20),
//     );
//     let range2 = PageNumRange::from_start_end(
//         VirtualPageNum::from_usize(12),
//         VirtualPageNum::from_usize(18),
//     );
//     assert!(range1.contains_range(&range2));
//     assert!(range2.contained_by(&range1));
// }

// #[test]
// fn test_page_num_range_iter() {
//     let range =
//         PageNumRange::from_start_end(VirtualPageNum::from_usize(3), VirtualPageNum::from_usize(6));
//     let mut iter = range.iter();
//     assert_eq!(iter.next().unwrap().as_usize(), 3);
//     assert_eq!(iter.next().unwrap().as_usize(), 4);
//     assert_eq!(iter.next().unwrap().as_usize(), 5);
//     assert!(iter.next().is_none());
// }

// #[test]
// fn test_address_range_iter() {
//     let range = AddressRange::from_start_end(
//         VirtualAddress::from_usize(0x1000),
//         VirtualAddress::from_usize(0x1003),
//     );
//     let mut iter = range.iter();
//     assert_eq!(iter.next().unwrap().as_usize(), 0x1000);
//     assert_eq!(iter.next().unwrap().as_usize(), 0x1001);
//     assert_eq!(iter.next().unwrap().as_usize(), 0x1002);
//     assert!(iter.next().is_none());
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_virtual_address_add_by() {
//         let va = VirtualAddress::from_usize(0x1000);
//         let va_added = va.add_by(0x1000);
//         assert_eq!(va_added.as_usize(), 0x2000);
//     }

//     #[test]
//     fn test_physical_address_minus_by() {
//         let pa = PhysicalAddress::from_usize(0x3000);
//         let pa_subtracted = pa.minus_by(0x1000);
//         assert_eq!(pa_subtracted.as_usize(), 0x2000);
//     }

//     #[test]
//     fn test_virtual_address_alignment() {
//         let va = VirtualAddress::from_usize(0x1234);
//         assert!(!va.is_aligned(0x1000));
//         let va_aligned = va.align_down(0x1000);
//         assert_eq!(va_aligned.as_usize(), 0x1000);
//         let va_aligned_up = va.align_up(0x1000);
//         assert_eq!(va_aligned_up.as_usize(), 0x2000);
//     }

//     #[test]
//     fn test_physical_address_alignment() {
//         let pa = PhysicalAddress::from_usize(0x1FFF);
//         assert!(!pa.is_page_aligned());
//         let pa_aligned = pa.align_page_down();
//         assert_eq!(pa_aligned.as_usize(), 0x1000);
//         let pa_aligned_up = pa.align_page_up();
//         assert_eq!(pa_aligned_up.as_usize(), 0x2000);
//     }

//     #[test]
//     fn test_address_range_contains() {
//         let start = VirtualAddress::from_usize(0x1000);
//         let end = VirtualAddress::from_usize(0x2000);
//         let range = AddressRange::from_start_end(start, end);
//         let addr_inside = VirtualAddress::from_usize(0x1800);
//         let addr_outside = VirtualAddress::from_usize(0x2000);
//         assert!(range.contains(addr_inside));
//         assert!(!range.contains(addr_outside));
//     }

//     #[test]
//     fn test_address_range_intersection() {
//         let range1 = AddressRange::from_start_end(
//             VirtualAddress::from_usize(0x1000),
//             VirtualAddress::from_usize(0x3000),
//         );
//         let range2 = AddressRange::from_start_end(
//             VirtualAddress::from_usize(0x2000),
//             VirtualAddress::from_usize(0x4000),
//         );
//         let intersection = range1.intersection(&range2).unwrap();
//         assert_eq!(intersection.start.as_usize(), 0x2000);
//         assert_eq!(intersection.end.as_usize(), 0x3000);
//     }

//     #[test]
//     fn test_address_range_union() {
//         let range1 = AddressRange::from_start_end(
//             VirtualAddress::from_usize(0x1000),
//             VirtualAddress::from_usize(0x2000),
//         );
//         let range2 = AddressRange::from_start_end(
//             VirtualAddress::from_usize(0x1500),
//             VirtualAddress::from_usize(0x2500),
//         );
//         let union = range1.union(&range2);
//         assert_eq!(union.start.as_usize(), 0x1000);
//         assert_eq!(union.end.as_usize(), 0x2500);
//     }

//     #[test]
//     fn test_virtual_page_num_operations() {
//         let vpn = VirtualPageNum::from_usize(10);
//         let vpn_added = vpn.add(5);
//         assert_eq!(vpn_added.as_usize(), 15);
//         let vpn_subtracted = vpn.minus(3);
//         assert_eq!(vpn_subtracted.as_usize(), 7);
//     }

//     #[test]
//     fn test_physical_page_num_operations() {
//         let ppn = PhysicalPageNum::from_usize(20);
//         let ppn_added = ppn.add(10);
//         assert_eq!(ppn_added.as_usize(), 30);
//         let ppn_subtracted = ppn.minus(5);
//         assert_eq!(ppn_subtracted.as_usize(), 15);
//     }

//     #[test]
//     fn test_page_num_range_iter() {
//         let range = PageNumRange::from_start_end(
//             VirtualPageNum::from_usize(0),
//             VirtualPageNum::from_usize(3),
//         );
//         let nums: Vec<_> = range.iter().map(|n| n.as_usize()).collect();
//         assert_eq!(nums, vec![0, 1, 2]);
//     }

//     #[test]
//     fn test_address_range_iter() {
//         let start = VirtualAddress::from_usize(0x1000);
//         let end = VirtualAddress::from_usize(0x1003);
//         let range = AddressRange::from_start_end(start, end);
//         let addrs: Vec<_> = range.iter().map(|addr| addr.as_usize()).collect();
//         assert_eq!(addrs, vec![0x1000, 0x1001, 0x1002]);
//     }

//     #[test]
//     fn test_virtual_address_as_ptr() {
//         let va = VirtualAddress::from_usize(0x1000);
//         let ptr = va.as_ptr::<u8>();
//         assert_eq!(ptr as usize, 0x1000);
//     }

// #[test]
// fn test_physical_address_as_ref() {
// let v = b'a';
// let pa = PhysicalAddress::from_ref(&v);
// let _val = pa.as_ref::<u8>();
// assert_eq!(*val, b'a');
// }

//     #[test]
//     fn test_address_range_methods() {
//         let start = VirtualAddress::from_usize(0x1000);
//         let len = 0x2000;
//         let range = AddressRange::from_start_len(start, len);
//         assert_eq!(range.len(), len);
//         assert_eq!(range.end().as_usize(), 0x3000);
//     }

//     #[test]
//     fn test_page_num_from_address() {
//         let addr = VirtualAddress::from_usize(0x12345);
//         let vpn_floor = VirtualPageNum::from_addr_floor(addr);
//         let vpn_ceil = VirtualPageNum::from_addr_ceil(addr);
//         assert_eq!(vpn_floor.as_usize(), 0x12);
//         assert_eq!(vpn_ceil.as_usize(), 0x13);
//     }

//     #[test]
//     fn test_virtual_page_num_start_end_addr() {
//         let vpn = VirtualPageNum::from_usize(0x10);
//         let start_addr = vpn.start_addr::<VirtualAddress>();
//         let end_addr = vpn.end_addr::<VirtualAddress>();
//         assert_eq!(start_addr.as_usize(), 0x10000);
//         assert_eq!(end_addr.as_usize(), 0x11000);
//     }
// }