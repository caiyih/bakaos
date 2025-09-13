use crate::symbol_addr;

#[repr(C)]
pub(crate) struct CpuLocalStorage {
    pub local_base: *mut u8,
    pub cpu_id: u32,
}

unsafe impl Sync for CpuLocalStorage {}

#[cfg(feature = "boot")]
#[link_section = ".cls"]
pub(crate) static CPU0: CpuLocalStorage = CpuLocalStorage {
    cpu_id: 0,
    local_base: core::ptr::null_mut(),
};

/// Get the base address of CPU local storage for the current CPU.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers and assumes
/// that the provided pointer is valid and points to a location within the
/// CPU local storage region.
#[inline(always)]
pub(super) unsafe fn get_cpu_local_base(ptr: *mut u8) -> *mut u8 {
    let vaddr = ptr as usize;
    let base = symbol_addr!(__scls) as usize;

    debug_assert!(vaddr >= base);

    let offset = vaddr - base;

    crate::baremetal::arch::current::cpu::get_cls_ptr()
        .as_ref()
        .local_base
        .add(offset)
}
