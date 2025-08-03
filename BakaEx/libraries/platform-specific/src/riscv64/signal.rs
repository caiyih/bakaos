use core::arch::global_asm;

#[cfg(all(target_arch = "riscv64", target_os = "none"))]
global_asm!(include_str!("signal.S"));
