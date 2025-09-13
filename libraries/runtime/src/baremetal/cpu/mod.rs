pub(crate) mod cls;
pub mod local;

#[cfg(feature = "boot")]
pub use boot_required::*;

#[cfg(feature = "boot")]
mod boot_required {
    use core::{
        ptr::{addr_of, NonNull},
        sync::atomic::AtomicU32,
    };

    use crate::{
        baremetal::{alloc_frame, cpu::cls::CpuLocalStorage},
        symbol_addr,
    };

    pub(crate) fn alloc_cpu_id() -> u32 {
        static NEXT_ID: AtomicU32 = AtomicU32::new(0);

        NEXT_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed)
    }

    pub unsafe fn current_cpu_id() -> usize {
        crate::baremetal::arch::current::cpu::get_cls_ptr()
            .as_ref()
            .cpu_id as usize
    }

    pub(crate) fn alloc_cpu_local_storage(cpuid: u32) -> NonNull<CpuLocalStorage> {
        let template_start = NonNull::new(symbol_addr!(__scls) as *mut u8).unwrap();
        let template_end = NonNull::new(symbol_addr!(__ecls) as *mut u8).unwrap();

        let cls_len = template_end.as_ptr() as usize - template_start.as_ptr() as usize;

        let layout = core::alloc::Layout::from_size_align(cls_len, 4096).unwrap();
        let cls = alloc_frame(layout);

        // Copy the template into the newly allocated memory
        unsafe { cls.copy_from_nonoverlapping(template_start, cls_len) };

        let desc_offset = addr_of!(super::cls::CPU0) as usize - template_start.as_ptr() as usize;

        let mut desc = unsafe { cls.add(desc_offset).cast::<CpuLocalStorage>() };

        let desc_mut = unsafe { desc.as_mut() };
        desc_mut.cpu_id = cpuid;
        desc_mut.local_base = cls.as_ptr();

        desc
    }
}
