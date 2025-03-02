use platform_specific::{legacy_println, phys_to_virt};

const HALT_ADDR: *mut u8 = phys_to_virt(0x100E001C) as *mut u8;

pub fn machine_shutdown(_failure: bool) -> ! {
    unsafe { HALT_ADDR.write_volatile(0x34) };
    loop {}
}

pub fn print_bootloader_info() {
    legacy_println!("Platform: loongarch64");
}
