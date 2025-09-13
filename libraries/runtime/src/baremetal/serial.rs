#[macro_export]
macro_rules! println {
    () => {{
        use core::fmt::Write;
        let mut serial = $crate::baremetal::arch::current::serial::SERIAL_PORT.lock();

        writeln!(serial).unwrap()
    }};

    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let mut serial = $crate::baremetal::arch::current::serial::SERIAL_PORT.lock();

        writeln!(serial, $($arg)*).unwrap()
    }};
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let mut serial = $crate::baremetal::arch::current::serial::SERIAL_PORT.lock();

        write!(serial, $($arg)*).unwrap()
    }};
}
