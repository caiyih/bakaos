use platform_specific::legacy_println;

const HALT_ADDR: *mut u8 = (0x8000_0000_0000_0000usize | 0x100E001C) as *mut u8;

#[no_mangle]
#[allow(clippy::empty_loop)]
pub fn machine_shutdown(_failure: bool) -> ! {
    unsafe { HALT_ADDR.write_volatile(0x34) };
    loop {}
}

pub fn print_bootloader_info() {
    legacy_println!("Platform: loongarch64");
}
