#![feature(linkage)]
#![feature(alloc_error_handler)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

use abstractions::operations::IUsizeAlias;
use address::VirtualAddressRange;
use buddy_system_allocator::LockedHeap;
use log::debug;

#[global_allocator]
static GLOBAL_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();

pub fn init(range: VirtualAddressRange) {
    unsafe {
        debug!("Initializing kernel heap: {range:#?}");

        GLOBAL_ALLOCATOR
            .lock()
            .init(range.start().as_usize(), range.len());
    }
}

// Returns in (requested, allocated, total)
pub fn heap_statistics() -> (usize, usize, usize) {
    let allocator = GLOBAL_ALLOCATOR.lock();

    (
        allocator.stats_alloc_user(),
        allocator.stats_alloc_actual(),
        allocator.stats_total_bytes(),
    )
}

#[linkage = "weak"]
#[no_mangle]
#[cfg(target_os = "none")]
#[alloc_error_handler]
fn __on_kernel_heap_oom(layout: core::alloc::Layout) -> ! {
    panic!(
        "Kernel heap is out of memory while allocating for layout: {:#?}",
        layout
    );
}
