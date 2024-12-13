// Helper module for CI tests
// This module is used to exit QEMU with a specific code, which is useful for CI tests.
// https://github.com/andre-richter/qemu-exit
use core::arch::asm;

use crate::legacy_println;

const EXIT_SUCCESS: u32 = 0x5555; // Equals `exit(0)`. qemu successful exit

const EXIT_FAILURE_FLAG: u32 = 0x3333;
const EXIT_FAILURE: u32 = exit_code_encode(1); // Equals `exit(1)`. qemu failed exit
const EXIT_RESET: u32 = 0x7777; // qemu reset

pub trait IQemuExitHandle {
    /// Exit with specified return code.
    ///
    /// Note: For `X86`, code is binary-OR'ed with `0x1` inside QEMU.
    fn exit(&self, code: u32) -> !;

    /// Exit QEMU using `EXIT_SUCCESS`, aka `0`, if possible.
    ///
    /// Note: Not possible for `X86`.
    fn exit_success(&self) -> !;

    /// Exit QEMU using `EXIT_FAILURE`, aka `1`.
    fn exit_failure(&self) -> !;
}

/// RISCV64 configuration
pub struct Rv64ExitHandle {
    /// Address of the sifive_test mapped device.
    addr: u64,
}

/// Encode the exit code using EXIT_FAILURE_FLAG.
const fn exit_code_encode(code: u32) -> u32 {
    (code << 16) | EXIT_FAILURE_FLAG
}

impl Rv64ExitHandle {
    /// Create an instance.
    pub const fn new(addr: u64) -> Self {
        Rv64ExitHandle { addr }
    }
}

impl IQemuExitHandle for Rv64ExitHandle {
    /// Exit qemu with specified exit code.
    fn exit(&self, code: u32) -> ! {
        // If code is not a special value, we need to encode it with EXIT_FAILURE_FLAG.
        let code = match code {
            EXIT_SUCCESS | EXIT_FAILURE | EXIT_RESET => code,
            _ => exit_code_encode(code),
        };

        unsafe {
            asm!(
                "sw {0}, 0({1})",
                in(reg)code, in(reg)self.addr
            );

            // For the case that the QEMU exit attempt did not work, transition into an infinite
            // loop. Calling `panic!()` here is unfeasible, since there is a good chance
            // this function here is the last expression in the `panic!()` handler
            // itself. This prevents a possible infinite loop.
            loop {
                asm!("wfi", options(nomem, nostack));
            }
        }
    }

    fn exit_success(&self) -> ! {
        self.exit(EXIT_SUCCESS);
    }

    fn exit_failure(&self) -> ! {
        self.exit(EXIT_FAILURE);
    }
}

const VIRT_TEST: u64 = 0x100000 | (constants::VIRT_ADDR_OFFSET as u64);

pub const QEMU_EXIT_HANDLE: Rv64ExitHandle = Rv64ExitHandle::new(VIRT_TEST);

pub fn is_ci_environment() -> bool {
    matches!(
        option_env!("CI_ENVIRONMENT"),
        Some("TRUE") | Some("True") | Some("true") | Some("1")
    )
}

pub fn exit_qemu_failure() -> ! {
    if is_ci_environment() {
        QEMU_EXIT_HANDLE.exit_failure();
    }

    legacy_println!(
        "Not running in CI environment. Should not happen. YOU SHOULD CHECK THE CALLSITE!"
    );

    #[allow(unreachable_code)]
    loop {
        unsafe {
            asm!("wfi", options(nomem, nostack));
        }
    }
}

pub fn exit_qemu_successfully() -> ! {
    if is_ci_environment() {
        QEMU_EXIT_HANDLE.exit_success();
    }

    legacy_println!(
        "Not running in CI environment. Should not happen. YOU SHOULD CHECK THE CALLSITE!"
    );

    #[allow(unreachable_code)]
    loop {
        unsafe {
            asm!("wfi", options(nomem, nostack));
        }
    }
}
