use constants::PROCESSOR_COUNT;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
pub(crate) struct KernelThreadContext {
    // loongArch need to save 10 static registers from $r22 to $r31
    pub ksv: [usize; 10],
    // The next instruction of eret when returning from trap
    // Which is usually in `return_to_user` funciton
    pub kra: usize,
    pub ksp: usize, // kernel stack pointer
    pub pt: usize,  // kernel page table
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
// This writes to $tp register
pub unsafe fn init_thread_info() {
    let u0 = platform_specific::r21();
    let p_ctx = &raw mut THREAD_CONTEXT_POOL[u0];

    ::core::arch::asm!("move $tp, {0}", in(reg) p_ctx);
}
