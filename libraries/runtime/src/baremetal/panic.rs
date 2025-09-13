use crate::{panic::PanicInfo, println};

#[panic_handler]
fn __rust_begin_unwind(info: &PanicInfo) -> ! {
    unsafe extern "Rust" {
        fn __panic_handler_impl(info: &PanicInfo) -> !;
    }

    unsafe { __panic_handler_impl(info) }
}

#[linkage = "weak"]
#[unsafe(no_mangle)]
unsafe extern "Rust" fn __panic_handler_impl(info: &PanicInfo) -> ! {
    println!("Kernel panic: {}", info.message());

    if let Some(location) = info.location() {
        println!("    at {}:{}", location.file(), location.line());
    };

    loop {}
}
