pub(crate) mod cls;
pub mod local;

use core::{
    ptr::{addr_of, NonNull},
    sync::atomic::AtomicU32,
};

use crate::baremetal::{alloc_frame, cpu::cls::CpuLocalStorage};

pub(crate) fn alloc_cpu_id() -> u32 {
    static NEXT_ID: AtomicU32 = AtomicU32::new(0);

    NEXT_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed)
}

pub unsafe fn current_cpu_id() -> usize {
    crate::baremetal::arch::current::cpu::get_cls_ptr()
        .as_ref()
        .cpu_id as usize
}

unsafe extern "C" {
    fn __scls();
    fn __ecls();
}

pub(crate) fn alloc_cpu_local_storage(cpuid: u32) -> NonNull<CpuLocalStorage> {
    let template_start = NonNull::new(__scls as *mut u8).unwrap();
    let template_end = NonNull::new(__ecls as *mut u8).unwrap();

    let cls_len = template_end.as_ptr() as usize - template_start.as_ptr() as usize;

    let layout = core::alloc::Layout::from_size_align(cls_len, 4096).unwrap();
    let cls = alloc_frame(layout);

    // Copy the template into the newly allocated memory
    unsafe { cls.copy_from_nonoverlapping(template_start, cls_len) };

    let desc_offset = addr_of!(cls::CPU0) as usize - template_start.as_ptr() as usize;

    let mut desc = unsafe { cls.add(desc_offset).cast::<CpuLocalStorage>() };

    let desc_mut = unsafe { desc.as_mut() };
    desc_mut.cpu_id = cpuid;
    desc_mut.local_base = cls.as_ptr();

    desc
}
