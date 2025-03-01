use core::mem::MaybeUninit;
use ns16550a::Uart;

use super::phys_to_virt;

// TODO: figure out if this is correct
// uart.put(c) never succeeds with the current implementation
const UART_BASE: usize = 0x1FE001E0;
static mut UART: MaybeUninit<Uart> = MaybeUninit::zeroed();

#[allow(static_mut_refs)]
// Don't rename, cross crates inter-operation
pub fn init_serial() {
    let base_va = phys_to_virt(UART_BASE);
    let uart = Uart::new(base_va);

    *unsafe { UART.assume_init_mut() } = uart;
}

#[no_mangle]
#[allow(static_mut_refs)]
pub fn console_writestr(str: &[u8]) {
    for &c in str {
        console_putchar(c);
    }
}

#[no_mangle]
#[inline(always)]
#[allow(static_mut_refs)]
pub fn console_putchar(c: u8) {
    let uart = unsafe { UART.assume_init_ref() };
    while uart.put(c).is_none() {}
}

#[no_mangle]
#[inline(always)]
#[allow(static_mut_refs)]
pub fn console_getchar() -> Option<u8> {
    unsafe { UART.assume_init_ref().get() }
}
