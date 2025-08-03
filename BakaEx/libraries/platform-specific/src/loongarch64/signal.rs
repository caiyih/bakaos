use core::arch::global_asm;

#[cfg(all(target_arch = "loongarch64", target_os = "none"))]
global_asm!(include_str!("signal.S"));
