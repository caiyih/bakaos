#[repr(C)]
pub(crate) struct CpuLocalStorage {
    pub local_base: *mut u8,
    pub cpu_id: u32,
}

unsafe extern "C" {
    fn __scls();
}

#[cfg(feature = "boot")]
#[link_section = ".cls"]
pub(crate) static mut CPU0: CpuLocalStorage = CpuLocalStorage {
    cpu_id: 0,
    local_base: core::ptr::null_mut(),
};

#[inline(always)]
pub(super) unsafe fn get_cpu_local_base(ptr: *mut u8) -> *mut u8 {
    let vaddr = ptr as usize;
    let base = __scls as usize;

    debug_assert!(vaddr >= base);

    let offset = vaddr - base;

    crate::baremetal::arch::current::cpu::get_cls_ptr()
        .as_ref()
        .local_base
        .add(offset)
}
