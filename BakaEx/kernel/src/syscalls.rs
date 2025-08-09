use platform_specific::{
    syscall_ids::{SYSCALL_ID_EXIT, SYSCALL_ID_WRITE},
    SyscallPayload,
};
use syscalls::{SyscallContext, SyscallResult};
use trap_abstractions::ISyscallPayload;

pub async fn handle_syscall_async(p: &SyscallPayload<'_, &SyscallContext>) -> SyscallResult {
    let ctx = p.payload;

    macro_rules! syscall {
        ($name:ident, $num_arg:tt) => {
            syscalls::syscall_internal!($num_arg, $name, ctx, p)
        };
    }

    match p.syscall_id() {
        SYSCALL_ID_WRITE => syscall!(sys_write, 3).await,
        SYSCALL_ID_EXIT => syscall!(sys_exit, 1),
        id => panic!("Unimplemented syscall: {}", id),
    }
}
