use ::core::arch::asm;

const LEGACY_PUTCHAR_EID: usize = 0x01;
const LEGACY_GETCHAR_EID: usize = 0x02;

pub fn console_writestr(str: &[u8]) {
    for c in str {
        console_putchar(*c);
    }
}

#[inline(always)]
pub fn console_putchar(c: u8) {
    unsafe {
        asm!(
            "ecall",
            in("a0") c as usize,
            in("a7") LEGACY_PUTCHAR_EID,
        );
    }
}

#[inline(always)]
pub fn console_getchar() -> Option<u8> {
    let mut ret: i8;

    unsafe {
        asm!(
            "ecall",
            lateout("a0") ret,
            in("a7") LEGACY_GETCHAR_EID,
        );
    }

    match ret {
        -1 => None,
        _ => Some(ret as u8),
    }
}
