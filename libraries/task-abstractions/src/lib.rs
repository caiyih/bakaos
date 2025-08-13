#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod flags;
mod id;
pub mod status;
mod task_id;

use alloc::{string::String, sync::Arc, vec::Vec};
use filesystem_abstractions::FileDescriptorTable;
use hermit_sync::SpinMutex;
pub use id::*;
use memory_space_abstractions::MemorySpace;
use mmu_abstractions::IMMU;
pub use task_id::*;
use trap_abstractions::ITaskTrapContext;

use crate::status::TaskStatus;

pub trait IProcess {
    fn pid(&self) -> u32;

    fn pgid(&self) -> u32;

    fn parent(&self) -> Option<Arc<dyn IProcess>>;

    fn threads(&self) -> Vec<Arc<dyn ITask>>;

    fn children(&self) -> Vec<Arc<dyn IProcess>>;

    fn memory_space(&self) -> &SpinMutex<MemorySpace>;

    fn mmu(&self) -> &SpinMutex<dyn IMMU>;

    fn fd_table(&self) -> &SpinMutex<FileDescriptorTable>;

    fn working_directory(&self) -> String;

    fn exit_code(&self) -> &SpinMutex<Option<u8>>;

    fn execve(&self, mem: MemorySpace, calling: u32);

    fn alloc_id(&self) -> TaskId;

    fn push_thread(&self, task: Arc<dyn ITask>);
}

pub trait ITask {
    fn tid(&self) -> u32;

    fn tgid(&self) -> u32;

    fn process(&self) -> &Arc<dyn IProcess>;

    fn status(&self) -> TaskStatus;

    fn update_status(&self, status: TaskStatus) -> TaskStatus;

    fn stats(&self) -> UserTaskStatistics;

    fn trap_context(&self) -> &dyn ITaskTrapContext;

    fn trap_context_mut(&self) -> &mut dyn ITaskTrapContext;

    fn fork_thread(&self) -> Arc<dyn ITask>;

    fn fork_process(&self) -> Arc<dyn ITask>;
}

#[derive(Debug, Clone, Default)]
pub struct UserTaskStatistics {
    pub external_interrupts: usize,
    pub timer_interrupts: usize,
    pub software_interrupts: usize,
    pub exceptions: usize,
    pub syscalls: usize,
}
