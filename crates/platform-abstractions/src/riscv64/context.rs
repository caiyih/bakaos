use constants::PROCESSOR_COUNT;

// Saved context for coroutine
// Following calling convention that only caller-saved registers are saved
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
struct CoroutineSavedContext {
    saved: [usize; 12], // 0 - 11
    kra: usize,         // kernel return address, 12
    ksp: usize,         // kernel sp, 13
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
struct KernelThreadContext {
    pub pt: usize, // kernel page table
    pub hartid: usize,
    pub ctx: CoroutineSavedContext,
}

static mut THREAD_CONTEXT_POOL: [KernelThreadContext; PROCESSOR_COUNT] =
    unsafe { core::mem::zeroed() };

#[expect(dead_code)]
pub(crate) fn set_kernel_page_table(pt: usize) {
    // page table must be aligned to 4K
    debug_assert!(pt % 4096 == 0);

    let cpu = platform_specific::current_processor_index();

    unsafe {
        THREAD_CONTEXT_POOL[cpu].pt = pt;
    }
}

// # Safety
// Can only be called once for each thread.
pub unsafe fn init_thread_info() {
    let hartid = platform_specific::tp();
    let p_ctx = unsafe { &raw mut THREAD_CONTEXT_POOL[hartid] };

    // initialize hart id in the thread context
    p_ctx.cast::<usize>().add(1).write_volatile(hartid);

    // Coroutine saved context
    let p_ctx = p_ctx.add(2) as usize;

    ::core::arch::asm!("mv tp, {}", in(reg) p_ctx);
}
