use core::cell::UnsafeCell;

use crate::baremetal::cpu::cls::get_cpu_local_base;

#[macro_export]
macro_rules! cpu_local {
    {} => {};

    {$(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = const $init:block; $($rest:tt)*} => {
        $crate::cpu_local_internal!($(#[$attr])* $vis $name, $t, const $init);
        $crate::cpu_local!($($rest)*);
    };

    {$(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = const $init:block} => (
        $crate::cpu_local_internal!($(#[$attr])* $vis $name, $t, const $init);
    );

    // process multiple declarations
    {$(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr; $($rest:tt)*} => (
        $crate::cpu_local_internal!($(#[$attr])* $vis $name, $t, $init);
        $crate::cpu_local!($($rest)*);
    );

    // handle a single declaration
    {$(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr} => (
        $crate::cpu_local_internal!($(#[$attr])* $vis $name, $t, $init);
    );
}

#[doc(hidden)]
#[macro_export]
macro_rules! cpu_local_internal {
    (@key $t:ty, $init:expr) => {{
        $crate::baremetal::cpu::local::CpuLocalVal::new($init)
    }};

    ($(#[$attr:meta])* $vis:vis $name:ident, $t:ty, $($init:tt)*) => {
        #[link_section = ".cls"]
        $(#[$attr])* $vis static $name: $crate::baremetal::cpu::local::CpuLocalVal<$t> =
            $crate::cpu_local_internal!(@key $t, $($init)*);
    };
}

#[doc(hidden)]
#[repr(transparent)]
pub struct CpuLocalVal<T> {
    val: UnsafeCell<T>,
}

unsafe impl<T> Send for CpuLocalVal<T> {}
unsafe impl<T> Sync for CpuLocalVal<T> {}

impl<T> CpuLocalVal<T> {
    pub const fn new(val: T) -> Self {
        CpuLocalVal {
            val: UnsafeCell::new(val),
        }
    }

    #[inline(always)]
    pub fn get_ptr(&self) -> *mut T {
        unsafe { get_cpu_local_base(self.val.get().cast()).cast::<T>() }
    }

    #[inline(always)]
    pub fn get(&self) -> &T {
        unsafe { self.get_ptr().as_ref().unwrap() }
    }

    #[inline(always)]
    pub unsafe fn get_mut(&mut self) -> &mut T {
        self.get_ptr().as_mut().unwrap()
    }
}
