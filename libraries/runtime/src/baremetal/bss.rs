use core::ptr::NonNull;

pub(crate) fn clear_bss() {
    unsafe extern "C" {
        fn __sbss();
        fn __ebss();
    }

    unsafe {
        clear_bss_range(
            NonNull::new(__sbss as usize as *mut u8).unwrap(),
            NonNull::new(__ebss as usize as *mut u8).unwrap(),
        )
    }
}

pub(crate) unsafe fn clear_bss_range(mut begin: NonNull<u8>, end: NonNull<u8>) {
    core::ptr::write_bytes(
        begin.as_mut(),
        0,
        end.as_ptr() as usize - begin.as_ptr() as usize,
    );
}
