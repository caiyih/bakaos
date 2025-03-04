#[cfg(any(target_arch = "riscv64", test))]
mod riscv64;

#[cfg(any(target_arch = "riscv64", test))]
pub use riscv64::*;
