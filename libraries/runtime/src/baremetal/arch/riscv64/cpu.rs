use core::ptr::NonNull;

use crate::baremetal::{arch::riscv64::registers::gr::get_tp, cpu::cls::CpuLocalStorage};

#[cfg(feature = "boot")]
pub(super) fn init() {
    use crate::baremetal::{
        arch::riscv64::registers::gr::set_tp,
        cpu::{alloc_cpu_id, alloc_cpu_local_storage},
    };

    fn store_tls_base(cls: NonNull<CpuLocalStorage>) {
        unsafe { set_tp(cls.as_ptr() as usize) };
    }

    let hartid = alloc_cpu_id();

    let cls = alloc_cpu_local_storage(hartid);

    store_tls_base(cls);
}

/// Gets the CPU-local storage pointer from the global pointer register.
///
/// # Safety
/// This function assumes that the TP register has been properly initialized
/// with a valid CpuLocalStorage pointer via `init()`.
#[inline]
pub(crate) fn get_cls_ptr() -> NonNull<CpuLocalStorage> {
    let ptr = get_tp() as *mut CpuLocalStorage;

    debug_assert!(!ptr.is_null(), "CPU local storage pointer is null");

    unsafe { NonNull::new_unchecked(ptr) }
}
