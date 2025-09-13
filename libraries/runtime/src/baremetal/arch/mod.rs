#[cfg(target_arch = "riscv64")]
pub mod riscv64;

pub mod current {
    #[cfg(target_arch = "riscv64")]
    pub use super::riscv64::*;
}
