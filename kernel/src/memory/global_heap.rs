use buddy_system_allocator::LockedHeap;
use log::debug;

#[link_section = ".bss.heap"]
static KERNEL_HEAP_START: [u8; 0] = [0; 0];

#[global_allocator]
static GLOBAL_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();

pub fn init() {
    unsafe {
        let start = KERNEL_HEAP_START.as_ptr() as usize;
        let len = constants::KERNEL_HEAP_SIZE;

        debug!(
            "Initializing kernel heap: {:#010x} - {:#010x}",
            start,
            start + len
        );

        GLOBAL_ALLOCATOR.lock().init(start, len);
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

#[alloc_error_handler]
fn __on_kernel_heap_oom(layout: core::alloc::Layout) -> ! {
    panic!(
        "Kernel heap is out of memory while allocating for layout: {:#?}",
        layout
    );
}
