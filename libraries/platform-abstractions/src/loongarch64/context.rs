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
}

static mut THREAD_CONTEXT_POOL: [KernelThreadContext; PROCESSOR_COUNT] =
    unsafe { core::mem::zeroed() };

// # Safety
// This writes to $tp register
pub unsafe fn init_thread_info() {
    let u0 = platform_specific::r21();
    let p_ctx = &raw mut THREAD_CONTEXT_POOL[u0];

    ::core::arch::asm!("move $tp, {0}", in(reg) p_ctx);
}
