#![no_std]

extern crate alloc;

use core::ops::{Deref, DerefMut};

use alloc::sync::Arc;
use memory_space::MemorySpace;
use task_abstractions::{IProcess, ITask};

pub trait ILinuxProcess: IProcess {
    fn execve(&self, mem: MemorySpace, calling: u32);
}

impl Deref for dyn ILinuxTask {
    type Target = dyn ITask;

    fn deref(&self) -> &Self::Target {
        self
    }
}

impl DerefMut for dyn ILinuxTask {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self
    }
}

pub trait ILinuxTask: ITask {
    fn linux_process(&self) -> Arc<dyn ILinuxProcess>;
}

impl Deref for dyn ILinuxProcess {
    type Target = dyn IProcess;

    fn deref(&self) -> &Self::Target {
        self
    }
}

impl DerefMut for dyn ILinuxProcess {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self
    }
}
