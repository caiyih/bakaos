use address::VirtualAddress;
use constants::ErrNo;
use threading::yield_now;
use timing::TimeSpec;

use crate::{SyscallContext, SyscallResult};

impl SyscallContext {
    pub async fn sys_nanosleep(&self, req: VirtualAddress, rem: VirtualAddress) -> SyscallResult {
        let process = self.task.process();

        let req = process
            .mmu()
            .lock()
            .import::<TimeSpec>(req)
            .map_err(|_| ErrNo::BadAddress)?;

        Self::check_time_validity(req)?;

        let start = self.kernel.time();

        // Signal not implemented yet, so
        let _interrupted = || {
            let now = self.kernel.time();

            if now < start + req {
                let remain = start + req - now;

                process
                    .mmu()
                    .lock()
                    .export::<TimeSpec>(rem, remain)
                    .map_err(|_| ErrNo::BadAddress)?;

                return Err(ErrNo::InterruptedSystemCall);
            }

            Ok(0isize)
        };

        while start + req > self.kernel.time() {
            yield_now().await;

            // TODO: check interrupt and call _interrupted()?
        }

        Ok(0)
    }

    fn check_time_validity(t: TimeSpec) -> Result<(), ErrNo> {
        // see man nanosleep
        if t.tv_sec < 0 || t.tv_nsec < 0 || t.tv_nsec > 999999999 {
            return Err(ErrNo::InvalidArgument);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;
    use std::time::SystemTime;

    use address::IAddressBase;
    use alloc::sync::Arc;
    use hermit_sync::SpinMutex;
    use memory_space_abstractions::MemorySpace;
    use mmu_abstractions::IMMU;
    use test_utilities::{
        allocation::segment::TestFrameAllocator, kernel::TestKernel, task::TestProcess,
    };
    use threading::block_on;

    use super::*;

    fn setup_syscall_context() -> (Arc<SpinMutex<dyn IMMU>>, SyscallContext) {
        let kernel = TestKernel::new().build();
        let (alloc, mmu) = TestFrameAllocator::new_with_mmu();

        let (_, task) = TestProcess::new()
            .with_memory_space(Some(MemorySpace::new(mmu.clone(), alloc)))
            .build();

        (mmu, SyscallContext::new(task, kernel))
    }

    #[test]
    fn test_syscall_bad_address() {
        let (_, ctx) = setup_syscall_context();

        let ret = block_on!(ctx.sys_nanosleep(VirtualAddress::null(), VirtualAddress::null()));

        assert_eq!(ret, Err(ErrNo::BadAddress));
    }

    #[test]
    fn test_syscall_sleep_enough() {
        let (mmu, ctx) = setup_syscall_context();

        let req = TimeSpec::new(1, 0);

        mmu.lock().register(&req, false);

        let before_call = SystemTime::now();
        let ret =
            block_on!(ctx.sys_nanosleep(VirtualAddress::from_ref(&req), VirtualAddress::null()));
        let after_call = SystemTime::now();

        assert_eq!(ret, Ok(0));

        let duration = after_call.duration_since(before_call).unwrap();
        assert!(duration >= std::time::Duration::from_secs(1));
    }

    #[test]
    fn test_syscall_sleep_not_too_long() {
        const THRESHOLD: Duration = Duration::from_secs(2);

        let (mmu, ctx) = setup_syscall_context();

        let req = TimeSpec::new(1, 0);

        mmu.lock().register(&req, false);

        let before_call = SystemTime::now();
        let ret =
            block_on!(ctx.sys_nanosleep(VirtualAddress::from_ref(&req), VirtualAddress::null()));
        let after_call = SystemTime::now();

        assert_eq!(ret, Ok(0));

        let duration = after_call.duration_since(before_call).unwrap();
        assert!(duration < THRESHOLD);
    }

    #[test]
    fn test_syscall_sec_negative() {
        let req = TimeSpec::new(-1, 0);

        test_syscall_invalid_argument(req);
    }

    #[test]
    fn test_syscall_nsec_negative() {
        let req = TimeSpec::new(0, -1);

        test_syscall_invalid_argument(req);
    }

    #[test]
    fn test_syscall_nsec_too_large() {
        let req = TimeSpec::new(0, 1_000_000_000);

        test_syscall_invalid_argument(req);
    }

    fn test_syscall_invalid_argument(req: TimeSpec) {
        let (mmu, ctx) = setup_syscall_context();

        mmu.lock().register(&req, false);

        let ret =
            block_on!(ctx.sys_nanosleep(VirtualAddress::from_ref(&req), VirtualAddress::null()));

        assert_eq!(ret, Err(ErrNo::InvalidArgument));
    }
}
