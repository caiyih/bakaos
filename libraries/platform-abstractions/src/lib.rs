#![no_std]
#![feature(linkage)]
#![feature(panic_can_unwind)]

extern crate alloc;

mod interrupts;
mod panic;

#[cfg(target_arch = "riscv64")]
mod riscv64;

#[cfg(target_arch = "riscv64")]
pub use riscv64::*;

#[cfg(target_arch = "loongarch64")]
pub mod loongarch64;

#[cfg(target_arch = "loongarch64")]
pub use loongarch64::*;

pub use interrupts::*;

#[cfg(any(target_arch = "riscv64", target_arch = "loongarch64"))]
pub(crate) unsafe fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }

    // After benchmarking, we got results below:
    // clear_bss_for_loop:
    //    ~160 ticks            iter 0
    //    ~40 ticks             iter 1 to 20
    // clear_bss_fast:
    //    ~203 ticks            iter 0
    //    ~2 ticks              iter 1 to 20
    // clear_bss_slice_fill:
    //    ~470 ticks            iter 0
    //    ~9 ticks              iter 1 to 20
    // We can see that clear_bss_for_loop is the fastest at the first iteration
    // Although clear_bss_fast is MUCH FASTER at the following iterations than it
    // Since We only have to clear bss once, we choose clear_bss_for_loop
    // This may be related to the CPU cache and branch prediction
    // because only the first iteration is affected the most
    // Also, we use u64 to write memory, which is faster than u8
    // And the compiler will actually unroll the loop by 2 times
    // So the actual loop writes 128 bits at a time
    clear_bss_for_loop(sbss as usize, ebss as usize);
}

#[allow(unused)]
pub(crate) unsafe fn clear_bss_for_loop(begin: usize, end: usize) {
    core::ptr::write_bytes(begin as *mut u8, 0, end - begin);
}
