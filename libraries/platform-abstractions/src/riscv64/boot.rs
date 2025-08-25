use ::core::arch::naked_asm;

use super::context::init_thread_info;
use crate::clear_bss;

#[unsafe(naked)]
#[no_mangle]
#[link_section = ".text.entry"]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        // FIXME: we use `go ${addr}` to boot in Vision Five 2
        // So these arguments were NOT passed by bootloader, and therefore we
        // have to manually set them here.
        // To prevent any undefined behavior, we set them to zero.
        "xor tp, tp, tp",
        "xor gp, gp, gp",
        // // Read the hart id
        // "mv tp, a0",
        // // Read the device tree address
        // "mv gp, a1",
        // Setup virtual memory
        // See comments below for details
        "la t0, {page_table}",
        "srli t0, t0, 12", // get the physical page number of PageTabe
        "li t1, 8 << 60",
        "or t0, t0, t1", // ppn | 8 << 60
        "csrw satp, t0",
        "sfence.vma",
        // jump to virtualized entry
        "li t1, {virt_addr_offset}",
        "la t0, {entry}",
        "or t0, t0, t1",
        // Do not save the return address to ra
        "jr t0",
        page_table = sym PAGE_TABLE,
        virt_addr_offset = const platform_specific::VIRT_ADDR_OFFSET,
        entry = sym _start_virtualized,
    )
}

#[unsafe(naked)]
#[no_mangle]
#[link_section = ".text.entry"]
unsafe extern "C" fn _start_virtualized() -> ! {
    naked_asm!(
        // Naver come back!
        "xor ra, ra, ra",
        // Clear fp so that unwind knows where to stop
        "xor fp, fp, fp",
        // Load the stack pointer after we entered the high half
        // The symbols are loaded with a fixed offset to PC
        // If we load the stack pointer before we entered the high half
        // The stack pointer will be in the low half, which is not what we want
        // But I still `or` the stack pointer with the offset to make the code more readable
        "la sp, __tmp_stack_top",
        "li t0, {virt_addr_offset}",
        "or sp, t0, sp",
        "call {pre_boot_init}",
        "j __kernel_start_main",
        virt_addr_offset = const platform_specific::VIRT_ADDR_OFFSET,
        pre_boot_init = sym pre_boot_init,
    )
}

// This basically includes two parts
//   1. Identity mapping of [0x40000000, 0x80000000) and [0x80000000, 0xc0000000)]
//   2. High half kernel mapping of
//      [ VIRTUAL_ADDRESS_OFFSET | 0x00000000, VIRTUAL_ADDRESS_OFFSET | 0x40000000)
//           to [0x00000000, 0x40000000)
//
//      [ VIRTUAL_ADDRESS_OFFSET | 0x40000000, VIRTUAL_ADDRESS_OFFSET | 0x80000000)
//           to [0x40000000, 0x80000000)
//
//      [ VIRTUAL_ADDRESS_OFFSET | 0x80000000, VIRTUAL_ADDRESS_OFFSET | 0xc0000000)
//           to [0x80000000, 0xc0000000)
//
// The first part is essential as the pc is still at the low half
// since satp is write until jump to virtualized entry
// But the two pages is not needed after the kernel entered the _start_virtualized
#[link_section = ".data.prepage"]
static mut PAGE_TABLE: [usize; 512] = {
    let mut arr: [usize; 512] = [0; 512];
    arr[1] = (0x40000 << 10) | 0xcf;
    arr[2] = (0x80000 << 10) | 0xcf;
    // Should be '(0x00000 << 10) | 0xcf' for clarifity
    // But Cargo clippy complains about this line, so i just write 0xcf here
    arr[0x100] = 0xcf;
    arr[0x101] = (0x40000 << 10) | 0xcf;
    arr[0x102] = (0x80000 << 10) | 0xcf;
    arr
};

unsafe extern "C" fn pre_boot_init() {
    unsafe { clear_bss() };

    init_thread_info();
}
