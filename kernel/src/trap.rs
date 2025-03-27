use alloc::sync::Arc;
use log::{trace, warn};
use platform_abstractions::UserInterrupt;
use platform_specific::{ISyscallContext, ISyscallContextMut};
use tasks::{TaskControlBlock, TaskStatus};

use crate::syscalls::{ISyscallResult, SyscallDispatcher};

pub async fn user_trap_handler_async(tcb: &Arc<TaskControlBlock>, return_reason: UserInterrupt) {
    match return_reason {
        UserInterrupt::Unknown(payload) => panic!("Unknown user trap occurred: {:?}", payload),
        UserInterrupt::Syscall => {
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
                        warn!(
                            "[Exception::Syscall] Handler for id: {} not found.",
                            syscall_id
                        );

                        0
                    }
                },
            };

            ctx.set_return_value(ret as usize);

            tcb.borrow_page_table().restore_temporary_modified_pages();
        }
        e => {
            log::warn!(
                "[Task: {}] User mode exeption occurred: {:?}, kernel killing it.",
                tcb.task_id.id(),
                e
            );

            *tcb.task_status.lock() = TaskStatus::Exited;
        }
    }
}
