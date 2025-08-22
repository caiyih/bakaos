#![feature(future_join)]
#![feature(const_trait_impl)]
#![cfg_attr(target_os = "none", no_std)]

use alloc::sync::Arc;
use constants::ErrNo;
use kernel_abstractions::IKernel;
use task_abstractions::ITask;

extern crate alloc;

pub mod sys_clone;
pub mod sys_execve;
pub mod sys_exit;
pub mod sys_mmap;
pub mod sys_nanosleep;
pub mod sys_sched_yield;
pub mod sys_uname;
pub mod sys_write;

pub type SyscallResult = Result<isize, ErrNo>;

pub trait ISyscallResult {
    fn as_usize(self) -> usize;
}

impl ISyscallResult for SyscallResult {
    fn as_usize(self) -> usize {
        match self {
            Ok(v) => v as usize,
            Err(e) => e as usize,
        }
    }
}

pub struct SyscallContext {
    #[allow(unused)]
    pub task: Arc<dyn ITask>,
    #[allow(unused)]
    pub kernel: Arc<dyn IKernel>,
}

impl SyscallContext {
    pub fn new(task: Arc<dyn ITask>, kernel: Arc<dyn IKernel>) -> SyscallContext {
        Self { task, kernel }
    }
}

#[doc(hidden)]
#[macro_export]
#[rustfmt::skip]
macro_rules! syscall_internal {
    (0, $name:ident, $ctx:ident, $p:ident) => {
        $ctx.$name()
    };
    (1, $name:ident, $ctx:ident, $p:ident) => {
        $ctx.$name($p.arg0())
    };
    (2, $name:ident, $ctx:ident, $p:ident) => {
        $ctx.$name($p.arg0(), $p.arg1())
    };
    (3, $name:ident, $ctx:ident, $p:ident) => {
        $ctx.$name($p.arg0(), $p.arg1(), $p.arg2())
    };
    (4, $name:ident, $ctx:ident, $p:ident) => {
        $ctx.$name($p.arg0(), $p.arg1(), $p.arg2(), $p.arg3())
    };
    (5, $name:ident, $ctx:ident, $p:ident) => {
        $ctx.$name($p.arg0(), $p.arg1(), $p.arg2(), $p.arg3(), $p.arg4())
    };
    (6, $name:ident, $ctx:ident, $p:ident) => {
        $ctx.$name($p.arg0(), $p.arg1(), $p.arg2(), $p.arg3(), $p.arg4(), $p.arg5())
    };
}
