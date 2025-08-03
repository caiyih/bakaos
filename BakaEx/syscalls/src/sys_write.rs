use crate::{SyscallContext, SyscallResult};
use address::VirtualAddress;
use alloc::sync::Arc;
use constants::ErrNo;
use filesystem_abstractions::IFile;
use threading::yield_now;

impl SyscallContext {
    pub async fn sys_write(&self, fd: usize, buf: VirtualAddress, count: usize) -> SyscallResult {
        log::debug!("sys_write: fd: {}, buf: {}, count: {}", fd, buf, count);

        let file = {
            let fd_table = self.task.process().fd_table().lock();

            fd_table.get(fd).ok_or(ErrNo::BadFileDescriptor)?.clone()
        };

        if !file.can_write() {
            return Err(ErrNo::BadFileDescriptor);
        }

        self.sys_write_internal(file, buf, count).await
    }

    async fn sys_write_internal(
        &self,
        file: Arc<dyn IFile>,
        buf: VirtualAddress,
        count: usize,
    ) -> SyscallResult {
        while !file.write_avaliable() {
            yield_now().await;
        }

        let mut bytes_written = 0;

        self.task
            .process()
            .mmu()
            .lock()
            .inspect_framed(buf, count, |buf, _| {
                bytes_written += file.write(buf);

                true
            })
            .map_err(|_| ErrNo::BadAddress)?;

        Ok(bytes_written as isize)
    }
}

#[cfg(test)]
mod tests {
    use abstractions::IUsizeAlias;
    use address::{IAddressBase, VirtualAddress};
    use alloc::vec::Vec;
    use allocation_abstractions::IFrameAllocator;
    use filesystem_abstractions::FileDescriptorTable;
    use hermit_sync::SpinMutex;
    use kernel_abstractions::IKernel;
    use memory_space_abstractions::MemorySpace;
    use mmu_abstractions::{GenericMappingFlags, PageSize, IMMU};
    use test_utilities::{
        allocation::contiguous::TestFrameAllocator, kernel::TestKernel, task::TestProcess,
    };
    use threading::block_on;

    use super::*;

    struct TestFile {
        bytes: SpinMutex<Vec<u8>>,
    }

    impl TestFile {
        pub fn new() -> Arc<TestFile> {
            Arc::new(Self {
                bytes: SpinMutex::new(Vec::new()),
            })
        }

        fn content(&self) -> Vec<u8> {
            self.bytes.lock().clone()
        }
    }

    impl IFile for TestFile {
        fn can_write(&self) -> bool {
            true
        }

        fn write_avaliable(&self) -> bool {
            true
        }

        fn write(&self, buf: &[u8]) -> usize {
            let mut bytes = self.bytes.lock();

            bytes.extend_from_slice(buf);

            buf.len()
        }
    }

    fn setup_kernel_with_memory() -> (
        Arc<dyn IKernel>,
        Arc<SpinMutex<dyn IFrameAllocator>>,
        Arc<SpinMutex<dyn IMMU>>,
    ) {
        const MEMORY_RANGE: usize = 1 * 1024 * 1024 * 1024; // 1 GB

        let (alloc, mmu) = TestFrameAllocator::new_with_mmu(MEMORY_RANGE);

        let kernel = TestKernel::new()
            .with_allocator(Some(alloc.clone()))
            .build();

        (kernel, alloc, mmu)
    }

    #[test]
    fn test_received_from_testenv() {
        let (kernel, alloc, mmu) = setup_kernel_with_memory();

        let test_file = TestFile::new();
        let mut fd_table = FileDescriptorTable::new();
        fd_table.allocate(test_file.clone());

        let (_, task) = TestProcess::new()
            .with_memory_space(Some(MemorySpace::new(mmu.clone(), alloc)))
            .with_fd_table(Some(fd_table))
            .build();

        let ctx = SyscallContext::new(task, kernel);

        let buf = b"Hello, world";
        mmu.lock().register(buf, false); // let the mmu know about the buffer

        let ret = block_on!(ctx.sys_write(0, buf.into(), buf.len()));

        assert_eq!(ret, Ok(buf.len() as isize));

        assert_eq!(test_file.content(), buf);
    }

    #[test]
    fn test_received_with_user_memory() {
        let (kernel, alloc, mmu) = setup_kernel_with_memory();

        let test_file = TestFile::new();
        let mut fd_table = FileDescriptorTable::new();
        fd_table.allocate(test_file.clone());

        let (_, task) = TestProcess::new()
            .with_memory_space(Some(MemorySpace::new(mmu.clone(), alloc.clone())))
            .with_fd_table(Some(fd_table))
            .build();

        // Let's allocate a frame and map it to the user memory space
        // We decide to map it to 0x1000000
        let frame = alloc.lock().alloc_frame().unwrap();
        let ptr = VirtualAddress::from_usize(0x1000000);
        mmu.lock()
            .map_single(
                ptr,
                frame.0,
                PageSize::_4K,
                GenericMappingFlags::User | GenericMappingFlags::Readable,
            )
            .unwrap();

        // Let's write something to the user memory space
        // We will write directly to the frame.
        let content = b"Hello, world";
        {
            let ptr = frame.0.as_usize() as *mut u8;
            let buf = unsafe { core::slice::from_raw_parts_mut(ptr, 4096) };

            buf[..content.len()].copy_from_slice(content);
        }

        let ctx = SyscallContext::new(task, kernel);

        // let's call sys_write with the virtual address
        let ret = block_on!(ctx.sys_write(0, ptr, content.len()));

        assert_eq!(ret, Ok(content.len() as isize));

        assert_eq!(test_file.content(), content);

        alloc.lock().dealloc(frame);
    }

    #[test]
    fn test_bad_fd_if_not_exist() {
        let (kernel, alloc, mmu) = setup_kernel_with_memory();

        let (_, task) = TestProcess::new()
            .with_fd_table(Some(FileDescriptorTable::new()))
            .with_memory_space(Some(MemorySpace::new(mmu.clone(), alloc)))
            .build();

        let ctx = SyscallContext::new(task, kernel);

        let buf = b"Hello, world";
        mmu.lock().register(buf, false); // let the mmu know about the buffer

        let ret = block_on!(ctx.sys_write(0, buf.into(), buf.len()));

        assert_eq!(ret, Err(ErrNo::BadFileDescriptor));
    }

    #[test]
    fn test_bad_address_with_invalid_buffer() {
        let (kernel, alloc, mmu) = setup_kernel_with_memory();

        let mut fd_table = FileDescriptorTable::new();
        fd_table.allocate(TestFile::new());

        let (_, task) = TestProcess::new()
            .with_fd_table(Some(fd_table))
            .with_memory_space(Some(MemorySpace::new(mmu.clone(), alloc)))
            .build();

        let ctx = SyscallContext::new(task, kernel);

        let ret = block_on!(ctx.sys_write(0, VirtualAddress::null(), 0));

        assert_eq!(ret, Err(ErrNo::BadAddress));
    }
}
