use core::{
    arch::asm,
    fmt::{self, Write},
};

pub trait IConsole: Write {
    #[allow(unused)]
    fn put_char(&self, c: u8) -> fmt::Result;

    #[allow(unused)]
    fn get_char(&self) -> u8;

    #[allow(unused)]
    fn name(&self) -> &'static str;
}

#[derive(Clone, Copy)]
pub struct LegacyConsole;

impl LegacyConsole {
    #[allow(unused)]
    fn get_api() -> Self {
        Self
    }
}

const LEGACY_PUTCHAR_EID: usize = 0x01;
const LEGACY_GETCHAR_EID: usize = 0x02;

impl Write for LegacyConsole {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.bytes() {
            self.put_char(c)?;
        }
        Ok(())
    }
}

impl LegacyConsole {
    #[allow(unused)]
    fn write_fmt(&mut self, args: fmt::Arguments) {
        Write::write_fmt(self, args).unwrap();
    }
}

impl IConsole for LegacyConsole {
    fn put_char(&self, c: u8) -> fmt::Result {
        unsafe {
            asm!(
                "ecall",
                in("a0") c as usize,
                in("a7") LEGACY_PUTCHAR_EID,
            );
        }
        Ok(())
    }

    fn get_char(&self) -> u8 {
        let mut ret: u8;

        unsafe {
            asm!(
                "ecall",
                lateout("a0") ret,
                in("a7") LEGACY_GETCHAR_EID,
            );
        }

        ret
    }

    fn name(&self) -> &'static str {
        "LegacyConsole"
    }
}
