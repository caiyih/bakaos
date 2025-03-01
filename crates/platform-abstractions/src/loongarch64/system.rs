use platform_specific::console_writestr;

pub fn machine_shutdown(_failure: bool) -> ! {
    // TODO: this is a stub
    loop {}
}

pub fn print_bootloader_info() {
    console_writestr(b"Platform: loongarch64");
}
