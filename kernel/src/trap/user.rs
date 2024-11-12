use super::set_kernel_trap_handler;

#[no_mangle]
extern "C" fn __user_trap_handler() -> ! {
    set_kernel_trap_handler();

    unimplemented!()
}
