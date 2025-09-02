use alloc::sync::Arc;
use allocation::FrameAllocator;
use allocation_abstractions::IFrameAllocator;
use filesystem_abstractions::DirectoryTreeNode;
use hermit_sync::SpinMutex;
use kernel_abstractions::{IKernel, IKernelSerial};
use linux_syscalls::SyscallContext;
use linux_task_abstractions::ILinuxTask;
use mmu_abstractions::IMMU;
use timing::TimeSpec;

use crate::serial::KernelSerial;

pub(crate) struct Kernel {
    serial: Arc<KernelSerial>,
    allocator: Arc<SpinMutex<FrameAllocator>>,
}

impl Kernel {
    pub fn new(serial: Arc<KernelSerial>, allocator: Arc<SpinMutex<FrameAllocator>>) -> Arc<Self> {
        Arc::new(Self { serial, allocator })
    }

    pub fn create_syscall_contenxt_for(
        self: &Arc<Self>,
        task: Arc<dyn ILinuxTask>,
    ) -> SyscallContext {
        SyscallContext {
            task,
            kernel: self.clone(),
        }
    }
}

impl IKernel for Kernel {
    fn serial(&self) -> Arc<dyn IKernelSerial> {
        self.serial.clone()
    }

    fn fs(&self) -> Arc<SpinMutex<Arc<DirectoryTreeNode>>> {
        todo!()
    }

    fn allocator(&self) -> Arc<SpinMutex<dyn IFrameAllocator>> {
        self.allocator.clone()
    }

    fn activate_mmu(&self, _pt: &dyn IMMU) {
        #[cfg_accessible(platform_specific::activate_pt)]
        platform_specific::activate_pt(_pt.platform_payload())
    }

    fn time(&self) -> TimeSpec {
        todo!()
    }
}
