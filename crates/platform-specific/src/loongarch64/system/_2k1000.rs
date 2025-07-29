#[unsafe(no_mangle)]
#[allow(clippy::empty_loop)]
extern "C" fn machine_shutdown(_failure: bool) -> ! {
    // We use reboot instead of poweroff to faster our debug.
    reboot();

    loop {}
}

const SYSCON_BASE: usize = 0x1fe27000 | 0x8000_0000_0000_0000;

#[allow(unused)]
fn reboot() {
    const REBOOT_OFFSET: usize = 0x30;
    const REBOOT_BASE: *mut u32 = (SYSCON_BASE + REBOOT_OFFSET) as *mut u32;

    const REBOOT_MASK: u32 = 0x00000001;

    unsafe {
        let mut val = REBOOT_BASE.read_volatile();
        val |= REBOOT_MASK;

        REBOOT_BASE.write_volatile(val);
    }
}

#[allow(unused)]
fn power_off() {
    const POWEROFF_OFFSET: usize = 0x14;
    const POWEROFF_BASE: *mut u32 = (SYSCON_BASE + POWEROFF_OFFSET) as *mut u32;

    const POWEROFF_MASK: u32 = 0x00003C00;
    const POWEROFF_VALUE: u32 = 0x00003C00;

    unsafe {
        let mut val = POWEROFF_BASE.read_volatile();
        val &= !POWEROFF_MASK;
        val |= POWEROFF_VALUE;

        POWEROFF_BASE.write_volatile(val);
    }
}
