use core::fmt::Write;

struct Console;

impl Write for Console {
    fn write_str(&mut self, _s: &str) -> core::fmt::Result {
        #[cfg(any(target_arch = "riscv64", target_arch = "loongarch64"))]
        {
            crate::console_writestr(_s.as_bytes());
            Ok(())
        }

        #[cfg(not(any(target_arch = "riscv64", target_arch = "loongarch64")))]
        Err(::core::fmt::Error)
    }
}

static mut CONSOLE: Console = Console;

#[inline(always)]
#[allow(static_mut_refs)]
pub fn __get_console() -> &'static mut dyn Write {
    unsafe { &mut CONSOLE }
}

#[macro_export]
macro_rules! legacy_print {
    ($($arg:tt)*) => {{
        ::core::write!($crate::__get_console(), $($arg)*).unwrap();
    }};
}

#[macro_export]
#[allow(unreachable_code)]
macro_rules! legacy_println {
    () => {
        ::core::writeln!($crate::__get_console()).unwrap();
    };
    ($($arg:tt)*) => {{
        ::core::writeln!($crate::__get_console(), $($arg)*).unwrap();
    }};
}
