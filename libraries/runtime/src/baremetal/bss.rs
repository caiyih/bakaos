use crate::symbol_addr;
use core::ptr::NonNull;

pub(crate) fn clear_bss() {
    unsafe {
        clear_bss_range(
            NonNull::new(symbol_addr!(__sbss) as *mut u8).unwrap(),
            NonNull::new(symbol_addr!(__ebss) as *mut u8).unwrap(),
        )
    }
}

/// Converts a pointer to a per-CPU local storage address.
///
/// # Safety
///
/// - `ptr` must be a valid pointer within the CLS region
/// - The caller must ensure that __scls has been properly initialized
/// - The current CPU's CLS must be properly set up via store_tls_base
pub(crate) unsafe fn clear_bss_range(mut begin: NonNull<u8>, end: NonNull<u8>) {
    core::ptr::write_bytes(
        begin.as_mut(),
        0,
        end.as_ptr() as usize - begin.as_ptr() as usize,
    );
}
