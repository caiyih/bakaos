use core::ops::Deref;

impl_addr!(VirtAddr,
    /// Represents a virtual address.
);

impl VirtAddr<'_> {
    /// Returns the address as a raw pointer of type `*const T`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the address is valid for reads of type `T`.
    /// Dereferencing the returned pointer is unsafe and may lead to undefined behavior
    /// if the address is not valid.
    ///
    /// A valid address not only requires to be non-null, but also must point to
    /// a properly mapped memory region in the **current** address space.
    #[inline(always)]
    pub unsafe fn as_ptr<T: Sized>(self) -> *const T {
        *self as *const T
    }

    /// Returns the address as a raw pointer of type `*mut T`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the address is valid for writes of type `T`.
    /// Dereferencing the returned pointer is unsafe and may lead to undefined behavior
    /// if the address is not valid.
    ///
    /// A valid address not only requires to be non-null, but also must point to
    /// a properly mapped memory region in the **current** address space.
    #[inline(always)]
    pub unsafe fn as_mut_ptr<T: Sized>(self) -> *mut T {
        *self as *mut T
    }
}

impl<T: ?Sized> From<*const T> for VirtAddr<'static> {
    #[inline(always)]
    fn from(ptr: *const T) -> Self {
        VirtAddr::new(ptr as *const () as usize)
    }
}

impl<T: ?Sized> From<*mut T> for VirtAddr<'static> {
    #[inline(always)]
    fn from(ptr: *mut T) -> Self {
        VirtAddr::new(ptr as *mut () as usize)
    }
}

impl<'a, T: ?Sized> From<&'a T> for VirtAddr<'a>
where
    T: Deref,
{
    #[inline(always)]
    default fn from(value: &'a T) -> Self {
        let inner = Deref::deref(value);

        inner.into()
    }
}

impl<'a, T: ?Sized> From<&'a T> for VirtAddr<'a> {
    #[inline(always)]
    default fn from(value: &'a T) -> Self {
        VirtAddr::new(value as *const T as *const () as usize)
    }
}

// There's no implementation for PhysAddr, as physical addresses are always static.
impl<'a> VirtAddr<'a> {
    /// Create a new address with the same lifetime as the given address.
    ///
    /// # Examples
    /// ```
    /// # use address_v2::VirtAddr;
    /// let val: i32 = 42;
    /// let vaddr1 = VirtAddr::from(&val); // local lifetime
    /// let vaddr2 = vaddr1.same_lifetime(0x1234); // same local lifetime
    ///
    /// let vaddr3 = VirtAddr::null; // static lifetime
    /// let vaddr4 = vaddr3.same_lifetime(0x5678); // static lifetime
    /// ```
    #[inline(always)]
    pub const fn same_lifetime(&'a self, addr: usize) -> VirtAddr<'a> {
        VirtAddr {
            _0: addr,
            _marker: self._marker,
        }
    }

    /// Promotes the address to a static lifetime.
    ///
    /// # Examples
    /// ```
    /// # use address_v2::VirtAddr;
    /// let val: i32 = 42;
    /// let vaddr = VirtAddr::from(&val); // local lifetime
    /// let static_vaddr = unsafe { vaddr.promote_to_static() }; // static lifetime
    /// ```
    /// # Safety
    /// The lifetime is explicitly limited, so promoting it to `'static` is unsafe.
    /// The caller must ensure that the address remains valid for the `'static` lifetime.
    #[inline(always)]
    pub const unsafe fn promote_to_static(self) -> VirtAddr<'static> {
        VirtAddr::null.same_lifetime(*self)
    }
}

#[cfg(test)]
mod virt_addr_tests {
    use core::fmt::Debug;

    use super::*;
    use crate::PhysAddr;

    #[test]
    fn test_virt_addr_creation() {
        let addr = VirtAddr::new(0x1000);
        assert_eq!(*addr, 0x1000);
        assert!(!addr.is_null());

        let null_addr = VirtAddr::null;
        assert!(null_addr.is_null());
        assert_eq!(*null_addr, 0);
    }

    #[test]
    fn test_virt_addr_arithmetic() {
        let addr1 = VirtAddr::new(0x1000);
        let addr2 = VirtAddr::new(0x2000);

        // Test virtual address arithmetic
        assert_eq!(addr1 + 0x1000usize, addr2);
        assert_eq!(addr2 - 0x1000usize, addr1);
        assert_eq!(addr2 - addr1, 0x1000isize);

        // Test pointer-like arithmetic
        let ptr_addr = VirtAddr::new(0x7fff_8000_0000);
        let offset_ptr = ptr_addr + 0x1000usize;
        assert_eq!(*offset_ptr, 0x7fff_8000_1000);
    }

    #[test]
    fn test_virt_addr_user_kernel_space() {
        // Test typical user space addresses (lower half)
        let user_addr = VirtAddr::new(0x0000_4000_0000_0000);
        assert!(*user_addr < 0x8000_0000_0000_0000);

        // Test typical kernel space addresses (upper half on x86_64)
        let kernel_addr = VirtAddr::new(0xFFFF_8000_0000_0000);
        assert!(*kernel_addr >= 0x8000_0000_0000_0000);

        // Test canonical addresses (important on x86_64)
        let canonical_user = VirtAddr::new(0x0000_7FFF_FFFF_FFFF);
        let canonical_kernel = VirtAddr::new(0xFFFF_8000_0000_0000);

        // These should be valid canonical addresses
        assert_eq!(*canonical_user, 0x0000_7FFF_FFFF_FFFF);
        assert_eq!(*canonical_kernel, 0xFFFF_8000_0000_0000);
    }

    #[test]
    fn test_virt_addr_page_operations() {
        // Test page-aligned virtual addresses
        let page_addr = VirtAddr::new(0x1000); // 4KB aligned
        assert_eq!(*page_addr & 0xFFF, 0);

        // Test getting page boundaries
        let unaligned_addr = VirtAddr::new(0x1234);
        let page_start = VirtAddr::new(*unaligned_addr & !0xFFF);
        let page_end = page_start + 0x1000usize;

        assert_eq!(*page_start, 0x1000);
        assert_eq!(*page_end, 0x2000);
        assert!(page_start <= unaligned_addr);
        assert!(unaligned_addr < page_end);
    }

    #[test]
    fn test_virt_addr_stack_heap() {
        // Test typical stack addresses (high user space)
        let stack_addr = VirtAddr::new(0x7fff_ffff_f000);

        // Test typical heap addresses (low user space)
        let heap_addr = VirtAddr::new(0x0000_0000_1000_0000);

        // Stack should be higher than heap in typical layouts
        assert!(stack_addr > heap_addr);

        // Test stack growth (downward)
        let stack_frame = stack_addr - 0x1000usize;
        assert!(stack_frame < stack_addr);
    }

    #[test]
    fn test_virt_addr_conversions() {
        let addr = VirtAddr::new(0xDEADBEEF);

        // Test conversion to usize
        let addr_usize: usize = addr.into();
        assert_eq!(addr_usize, 0xDEADBEEF);

        // Test conversion from usize
        let addr_from: VirtAddr = VirtAddr::from(0xCAFEBABE);
        assert_eq!(*addr_from, 0xCAFEBABE);

        // Test that VirtAddr and PhysAddr are distinct types
        let vaddr = VirtAddr::new(0x1000);
        let paddr = PhysAddr::new(0x1000);

        // They should have the same underlying value but be different types
        assert_eq!(*vaddr, *paddr);

        // Test that they can't be directly compared (this would be a compile error)
        // assert_eq!(vaddr, paddr); // This should not compile
    }

    #[test]
    fn test_virt_addr_formatting() {
        let addr = VirtAddr::new(0x12345678);

        let debug_str = format!("{:?}", addr);
        assert_eq!(debug_str, "VirtAddr(0x12345678)");

        let display_str = format!("{}", addr);
        assert_eq!(display_str, "VirtAddr(0x12345678)");
    }

    #[test]
    fn test_virt_addr_collections() {
        use std::collections::BTreeMap;

        let mut address_map = BTreeMap::new();

        // Map virtual addresses to permissions or regions
        address_map.insert(VirtAddr::new(0x400000), "text");
        address_map.insert(VirtAddr::new(0x600000), "data");
        address_map.insert(VirtAddr::new(0x800000), "heap");
        address_map.insert(VirtAddr::new(0x7fff_0000_0000), "stack");

        assert_eq!(address_map.get(&VirtAddr::new(0x400000)), Some(&"text"));
        assert_eq!(address_map.len(), 4);

        // Test that addresses are properly ordered
        let addrs: Vec<_> = address_map.keys().copied().collect();
        assert_eq!(
            addrs,
            vec![
                VirtAddr::new(0x400000),
                VirtAddr::new(0x600000),
                VirtAddr::new(0x800000),
                VirtAddr::new(0x7fff_0000_0000),
            ]
        );
    }

    #[test]
    fn test_virt_addr_pointer_arithmetic() {
        let base = VirtAddr::new(0x10000);

        // Test array-like access patterns
        let element_size = 8usize; // 8-byte elements
        let elements = [
            base,
            base + element_size,
            base + element_size * 2,
            base + element_size * 3,
        ];

        assert_eq!(*elements[0], 0x10000);
        assert_eq!(*elements[1], 0x10008);
        assert_eq!(*elements[2], 0x10010);
        assert_eq!(*elements[3], 0x10018);

        // Test distance calculation
        assert_eq!(elements[3] - elements[0], element_size as isize * 3);
    }

    fn test_virt_addr_as_ptr_scene<T: Eq + Debug>(expected: T, action: impl Fn() -> T) {
        assert_eq!(expected, action());
    }

    #[test]
    fn test_test_virt_addr_as_ptr() {
        test_virt_addr_as_ptr_scene(42 as *const i32, || {
            let addr = VirtAddr::new(42);
            unsafe { addr.as_ptr() }
        });

        test_virt_addr_as_ptr_scene(std::ptr::null_mut::<i32>(), || {
            let null_addr = VirtAddr::null;
            unsafe { null_addr.as_mut_ptr() }
        });
    }

    #[test]
    fn test_value_into() {
        let value: i32 = 42;

        // let addr: VirtAddr = (&value).into(); // equivalent to

        let addr: VirtAddr = From::from(&value);

        assert_eq!(*addr, &value as *const _ as usize);
    }

    #[test]
    fn test_slice_into() {
        let bytes: &[i32] = [0x12, 0x34, 0x56, 0x78].as_slice();

        let addr: VirtAddr = bytes.into();

        assert_eq!(*addr, bytes.as_ptr() as usize);
    }

    #[test]
    fn test_inline_array_ref_into() {
        let bytes: &[i32; 4] = &[0x12, 0x34, 0x56, 0x78];

        let addr: VirtAddr = bytes.into();

        assert_eq!(*addr, bytes.as_ptr() as usize);
    }

    #[test]
    fn test_inline_array_into() {
        // inline array is basically a big struct
        let bytes: [i32; 4] = [0x12, 0x34, 0x56, 0x78];

        let addr: VirtAddr = From::from(&bytes);

        assert_eq!(*addr, bytes.as_ptr() as usize);
    }

    #[test]
    fn test_boxed_value_into() {
        let boxed = Box::new(42);

        let addr: VirtAddr = boxed.as_ref().into();

        assert_eq!(*addr, boxed.as_ref() as *const _ as usize);
    }

    #[test]
    fn test_boxed_value_from() {
        let boxed = Box::new(42);

        let addr: VirtAddr = From::from(&boxed);

        assert_eq!(*addr, boxed.as_ref() as *const _ as usize);
    }

    #[test]
    fn test_boxed_slice_val_into() {
        let slice = vec![0x12, 0x34, 0x56, 0x78];

        let boxed: Box<[i32]> = slice.into_boxed_slice();

        let addr: VirtAddr = boxed.as_ref().into(); // expect addr to be the address of the slice, not the box

        assert_eq!(*addr, boxed.deref().as_ptr() as usize);
    }

    #[test]
    fn test_boxed_slice_ref_into() {
        static SLICE: &[i32] = &[0x12, 0x34, 0x56, 0x78];

        let boxed = Box::new(SLICE);

        let addr: VirtAddr = boxed.as_ref().into(); // expect addr to be the address of the slice's first value
                                                    // not the box or the boxed reference

        assert_eq!(*addr, boxed.deref().as_ptr() as usize);
    }

    #[test]
    fn test_boxed_inline_array_into() {
        let boxed: Box<[i32; 4]> = Box::new([0x12, 0x34, 0x56, 0x78]);

        let addr: VirtAddr = boxed.as_ref().into();

        assert_eq!(*addr, boxed.deref().as_ptr() as usize);
    }

    #[test]
    fn test_boxed_inline_array_ref_into() {
        static ARRAY: [i32; 4] = [0x12, 0x34, 0x56, 0x78];

        let boxed: Box<&[i32; 4]> = Box::new(&ARRAY);

        let addr: VirtAddr = boxed.as_ref().into();

        assert_eq!(*addr, boxed.deref().as_ptr() as usize);
    }

    #[test]
    fn test_same_lifetime() {
        fn foo<'a>(lhs: VirtAddr<'a>, rhs: VirtAddr<'a>) {
            use core::hint::black_box;

            black_box((lhs, rhs));
        }

        let val = 42;

        let null = VirtAddr::from(&val);
        let addr = null.same_lifetime(0x10000);

        foo(null, addr);
    }

    #[test]
    fn test_promote_to_static() {
        fn take_static(addr: VirtAddr<'static>) {
            use core::hint::black_box;

            black_box(addr);
        }

        let addr = {
            let val = 24;
            unsafe { VirtAddr::from(&val).promote_to_static() }
        };

        assert!(!addr.is_null());

        let val = 42;

        let local = VirtAddr::from(&val);
        let static_addr = unsafe { local.promote_to_static() };

        take_static(static_addr);

        assert_eq!(*local, *static_addr);
        assert_eq!(local, static_addr); // TODO: Not sure if we should allow this
    }
}
