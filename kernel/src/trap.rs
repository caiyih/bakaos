use alloc::{format, string::String, sync::Arc};
use filesystem_abstractions::{FileSystemError, FileSystemResult, IInode, InodeMetadata};
use log::{trace, warn};
use platform_abstractions::UserInterrupt;
use platform_specific::{ISyscallContext, ISyscallContextMut};
use tasks::{TaskControlBlock, TaskStatus};

use crate::{
    kernel::kernel_metadata,
    syscalls::{ISyscallResult, SyscallDispatcher},
};

pub async fn user_trap_handler_async(tcb: &Arc<TaskControlBlock>, return_reason: UserInterrupt) {
    let kernel_stat = kernel_metadata().stat();

    match return_reason {
        UserInterrupt::Unknown(payload) => panic!("Unknown user trap occurred: {:?}", payload),
        UserInterrupt::Syscall => {
            kernel_stat.on_syscall();

            let mut ctx = tcb.to_syscall_context();

            ctx.move_to_next_instruction();

            let syscall_id = ctx.syscall_id();
            let ret = match SyscallDispatcher::dispatch(syscall_id) {
                Some(handler) => {
                    trace!(
                        "[Exception::Syscall] [Task {}({})] Sync handler name: {}({})",
                        ctx.task_id.id(),
                        tcb.pcb.lock().id,
                        handler.name(),
                        syscall_id,
                    );

                    handler.handle(&mut ctx).to_ret()
                }
                None => match SyscallDispatcher::dispatch_async(&mut ctx, syscall_id).await {
                    Some(res) => res.to_ret(),
                    None => {
                        warn!("[Exception::Syscall] Handler for id: {syscall_id} not found.");

                        0
                    }
                },
            };

            ctx.set_return_value(ret as usize);

            tcb.borrow_page_table().restore_temporary_modified_pages();
        }
        e => {
            let pcb = tcb.pcb.lock();
            log::warn!(
                "[Task: {}] User mode exeption occurred: {:?}, kernel killing it. Commandline: \"{} {:?}\".Memory space: \n{:#018x?}\nTrap Context: \n{:#018x?}",
                tcb.task_id.id(),
                e,
                pcb.executable,
                pcb.command_line,
                pcb.memory_space.mappings(),
                unsafe { tcb.trap_context.get().as_ref().unwrap() },
            );

            *tcb.task_status.lock() = TaskStatus::Exited;
        }
    }
}

pub(crate) struct ProcInterrputsInode;

impl ProcInterrputsInode {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Arc<dyn IInode> {
        Arc::new(ProcInterrputsInode)
    }

    fn generate_content(&self) -> String {
        let mut content = String::new();
        let mut vector_id = 0;

        macro_rules! add_field {
            ($count:expr) => {
                content.push_str(&format!("{}: {}\n", vector_id, $count));

                vector_id += 1;
            };
        }

        let kernel_stat = kernel_metadata().stat();

        add_field!(kernel_stat.syscall_count());
        add_field!(kernel_stat.timer_interrupts());
        add_field!(kernel_stat.software_interrupts());
        // TODO: Add more

        let _ = vector_id; // supress unused assignment warning

        content
    }
}

impl IInode for ProcInterrputsInode {
    fn metadata(&self) -> InodeMetadata {
        InodeMetadata {
            filename: "interrupts",
            entry_type: filesystem_abstractions::DirectoryEntryType::File,
            size: 0, // Linux treat this as an empty file unless you read it
        }
    }

    fn readat(&self, offset: usize, buffer: &mut [u8]) -> FileSystemResult<usize> {
        let content = self.generate_content();
        let bytes = content.as_bytes();

        if offset >= bytes.len() {
            return Ok(0);
        }

        let bytes_to_copy = bytes.len() - offset;

        buffer[..bytes_to_copy].copy_from_slice(&bytes[offset..]);

        Ok(bytes_to_copy)
    }

    fn writeat(&self, _offset: usize, _buffer: &[u8]) -> FileSystemResult<usize> {
        Err(FileSystemError::NotPermitted)
    }

    fn renaming(&self, _new_name: &str) -> FileSystemResult<()> {
        Err(FileSystemError::NotPermitted)
    }

    fn removing(&self) -> FileSystemResult<()> {
        Err(FileSystemError::NotPermitted)
    }
}
