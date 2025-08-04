use platform_specific::{
    syscall_ids::{SYSCALL_ID_EXIT, SYSCALL_ID_WRITE},
    SyscallPayload,
};
use syscalls::{SyscallContext, SyscallResult};
use trap_abstractions::ISyscallPayload;

pub async fn handle_syscall_async(p: &SyscallPayload<'_, &SyscallContext>) -> SyscallResult {
    let ctx = p.payload;

    match p.syscall_id() {
        SYSCALL_ID_WRITE => ctx.sys_write(p.arg0(), p.arg1(), p.arg2()).await,
        SYSCALL_ID_EXIT => ctx.sys_exit(p.arg0()),
        id => panic!("Unimplemented syscall: {}", id),
    }
}
