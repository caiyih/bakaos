#[cfg(any(target_arch = "riscv64", all(test, feature = "riscv64")))]
mod riscv64;

#[cfg(any(target_arch = "riscv64", all(test, feature = "riscv64")))]
pub use riscv64::*;

#[cfg(any(target_arch = "loongarch64", all(test, feature = "loongarch64")))]
mod loongarch64;

#[cfg(any(target_arch = "loongarch64", all(test, feature = "loongarch64")))]
pub use loongarch64::*;
