#[cfg(target_arch = "riscv64")]
pub mod riscv64;

pub mod current {
    #[cfg(target_arch = "riscv64")]
    pub use super::riscv64::*;

    #[cfg(not(feature = "boot"))]
    pub mod cpu {
        use crate::baremetal::cpu::cls::CpuLocalStorage;
        use core::ptr::NonNull;

        pub(crate) fn get_cls_ptr() -> NonNull<CpuLocalStorage> {
            NonNull::dangling()
        }
    }
}
