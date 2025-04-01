use core::ptr::addr_of;

use address::{VirtualAddress, VirtualAddressRange};

#[link_section = ".bss.heap"]
static KERNEL_HEAP_START: [u8; 0] = [0; 0];

pub fn init() {
    global_heap::init(VirtualAddressRange::from_start_len(
        VirtualAddress::from_ptr(addr_of!(KERNEL_HEAP_START)),
        constants::KERNEL_HEAP_SIZE,
    ));
}
