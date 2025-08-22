use platform_specific::legacy_println;

unsafe extern "C" {
    pub unsafe fn machine_shutdown(failure: bool) -> !;
}

pub fn print_bootloader_info() {
    legacy_println!("Platform: loongarch64");
}
