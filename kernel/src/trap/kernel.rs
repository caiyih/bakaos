use super::__return_from_kernel_trap;

#[no_mangle]
extern "C" fn __kernel_trap_handler() -> ! {
    unsafe { __return_from_kernel_trap() };
}
