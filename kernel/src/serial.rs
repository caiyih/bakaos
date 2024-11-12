use core::ptr::addr_of_mut;

use crate::firmwares::console::{IConsole, LegacyConsole};

pub static mut LEGACY_INTERFACE: LegacyConsole = LegacyConsole;

#[inline(always)]
pub(crate) fn legacy_console() -> &'static mut LegacyConsole {
    unsafe { addr_of_mut!(LEGACY_INTERFACE).as_mut().unwrap() }
}

// Legacy_x macros provides a way to print to the legacy console
// While print and println macros uses dynamic dispatch, which must be initialized and used after memory is initialized

#[macro_export]
macro_rules! legacy_print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        write!($crate::serial::legacy_console(), $($arg)*).unwrap();
    }};
}

#[macro_export]
#[allow(unreachable_code)]
macro_rules! legacy_println {
    () => {
        use core::fmt::Write;
        writeln!($crate::serial::legacy_console()).unwrap();
    };
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        writeln!($crate::serial::legacy_console(), $($arg)*).unwrap();
    }};
}

#[allow(unused)]
pub fn legacy_putchar(c: u8) {
    legacy_console().put_char(c).unwrap();
}
