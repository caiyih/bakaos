use buddy_system_allocator::LockedHeap;

#[link_section = ".bss.heap"]
static KERNEL_HEAP_START: [u8; 0] = [0; 0];

#[global_allocator]
static GLOBAL_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();

pub fn init() {
    unsafe {
        let start = KERNEL_HEAP_START.as_ptr() as usize;
        let len = 0x00800000; // refert to linker script

        GLOBAL_ALLOCATOR.lock().init(start, len);
    }
}

#[alloc_error_handler]
fn __on_kernel_heap_oom(layout: core::alloc::Layout) -> ! {
    panic!(
        "Kernel heap is out of memory while allocating for layout: {:#?}",
        layout
    );
}
