use core::ptr::NonNull;

use crate::baremetal::{
    arch::riscv64::registers::gr::{get_gp, set_gp},
    cpu::{alloc_cpu_id, alloc_cpu_local_storage, cls::CpuLocalStorage},
};

pub(super) fn init() {
    let hartid = alloc_cpu_id();

    let cls = alloc_cpu_local_storage(hartid);

    store_tls_base(cls);
}

fn store_tls_base(cls: NonNull<CpuLocalStorage>) {
    unsafe { set_gp(cls.as_ptr() as usize) };
}

#[inline]
pub(crate) fn get_cls_ptr() -> NonNull<CpuLocalStorage> {
    unsafe { NonNull::new_unchecked(get_gp() as *mut CpuLocalStorage) }
}
