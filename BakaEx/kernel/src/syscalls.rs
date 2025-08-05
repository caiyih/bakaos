use platform_specific::{
    syscall_ids::{SYSCALL_ID_EXIT, SYSCALL_ID_WRITE},
    SyscallPayload,
};
use syscalls::{SyscallContext, SyscallResult};
use trap_abstractions::ISyscallPayload;

pub async fn handle_syscall_async(p: &SyscallPayload<'_, &SyscallContext>) -> SyscallResult {
    let ctx = p.payload;

    macro_rules! syscall {
        ($num:tt, $name:ident) => {
            syscalls::syscall_internal!($num, $name, ctx, p)
        };
    }

    match p.syscall_id() {
        SYSCALL_ID_WRITE => syscall!(3, sys_write).await,
        SYSCALL_ID_EXIT => syscall!(1, sys_exit),
        id => panic!("Unimplemented syscall: {}", id),
    }
}
