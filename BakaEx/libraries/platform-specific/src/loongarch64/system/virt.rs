pub fn boot_init() {
    // loongson,ls7a-rtc
    // https://github.com/qemu/qemu/blob/661c2e1ab29cd9c4d268ae3f44712e8d421c0e56/include/hw/pci-host/ls7a.h#L45
    const RTC_BASE: usize = 0x10000000 + 0x00080000 + 0x00050100;
    const SYS_RTCCTRL: usize = 0x40;

    const RTC_MASK: u64 = ((!0u64) >> (64 - (1))) << (13);
    const TOY_MASK: u64 = ((!0u64) >> (64 - (1))) << (11);
    const EO_MASK: u64 = ((!0u64) >> (64 - (1))) << (8);

    let rtc_ctrl = ((RTC_BASE + SYS_RTCCTRL) | 0x8000_0000_0000_0000) as *mut u32;
    unsafe {
        rtc_ctrl.write_volatile((TOY_MASK | EO_MASK | RTC_MASK) as u32);
    }
}

#[no_mangle]
#[allow(clippy::empty_loop)]
extern "C" fn machine_shutdown(_failure: bool) -> ! {
    const HALT_BASE: usize = 0x100E001C;
    const HALT_ADDR: *mut u8 = (HALT_BASE | 0x8000_0000_0000_0000usize) as *mut u8;

    unsafe { HALT_ADDR.write_volatile(0x34) };
    loop {}
}
