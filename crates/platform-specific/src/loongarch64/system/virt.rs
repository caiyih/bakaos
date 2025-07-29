pub fn boot_init() {}

#[no_mangle]
#[allow(clippy::empty_loop)]
extern "C" fn machine_shutdown(_failure: bool) -> ! {
    const HALT_BASE: usize = 0x100E001C;
    const HALT_ADDR: *mut u8 = (HALT_BASE | 0x8000_0000_0000_0000usize) as *mut u8;

    unsafe { HALT_ADDR.write_volatile(0x34) };
    loop {}
}
