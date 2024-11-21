use core::mem::size_of;

use abstractions::IUsizeAlias;

use crate::*;

pub type PhysicalAddressRange = AddressRange<PhysicalAddress>;

impl_range_display!(PhysicalAddressRange);

impl PhysicalAddressRange {
    pub fn to_virtual(&self) -> VirtualAddressRange {
        VirtualAddressRange::from_start_end(self.start().to_virtual(), self.end().to_virtual())
    }
}

impl PhysicalAddressRange {
    pub fn as_slice<T>(&self) -> &'static [T] {
        debug_assert!(self.start().is_aligned(size_of::<T>()));
        debug_assert!(self.end().is_aligned(size_of::<T>()));

        unsafe {
            core::slice::from_raw_parts(
                self.start().as_usize() as *const T,
                self.len() / size_of::<T>(),
            )
        }
    }

    pub fn as_mut_slice<T>(&self) -> &'static mut [T] {
        debug_assert!(self.start().is_aligned(size_of::<T>()));
        debug_assert!(self.end().is_aligned(size_of::<T>()));

        unsafe {
            core::slice::from_raw_parts_mut(
                self.start().as_usize() as *mut T,
                self.len() / size_of::<T>(),
            )
        }
    }

    pub fn from_slice<T>(slice: &[T]) -> Self {
        let start = PhysicalAddress::from_ptr(slice.as_ptr());
        let end = start.add_n::<T>(slice.len());
        PhysicalAddressRange::from_start_end(start, end)
    }
}

#[cfg(test)]
mod physical_address_range_tests {
    use core::mem::size_of;

    use abstractions::IUsizeAlias;

    use super::PhysicalAddressRange;

    #[test]
    fn test_from_slice() {
        let buf = [42i32; 50];
        let start_addr = buf.as_ptr() as usize;
        let addr_range = buf.len() * size_of::<i32>();

        let range = PhysicalAddressRange::from_slice(&buf);

        assert_eq!(range.start().as_usize(), start_addr);
        assert_eq!(range.len(), addr_range);
    }

    #[test]
    fn test_as_slice() {
        let buf = [42i32; 50];
        let range = PhysicalAddressRange::from_slice(&buf);

        let slice = range;

        for item in slice.as_slice::<i32>() {
            assert_eq!(*item, 42);
        }
    }

    #[test]
    #[should_panic]
    fn test_as_slice_alignment() {
        let buf = [0u8, 33];
        let range = PhysicalAddressRange::from_slice(&buf);

        let _ = range.as_slice::<i32>();
    }

    #[test]
    fn test_as_mut_slice() {
        let buf = [0i32; 50];
        let range = PhysicalAddressRange::from_slice(&buf);

        for item in range.as_mut_slice::<i32>() {
            *item = 42;
        }

        for item in range.as_slice::<i32>() {
            assert_eq!(*item, 42);
        }
    }
}
