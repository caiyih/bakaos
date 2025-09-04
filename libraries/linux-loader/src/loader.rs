use abstractions::IUsizeAlias;
use address::{IAlignableAddress, VirtualAddress};
use alloc::{fmt::Debug, string::String, sync::Arc, vec::Vec};
use allocation_abstractions::IFrameAllocator;
use filesystem_abstractions::{DirectoryTreeNode, IInode};
use hermit_sync::SpinMutex;
use log::trace;
use memory_space::MemorySpace;
use mmu_abstractions::IMMU;

use crate::{auxv::*, ProcessContext};

// A data structure to build a memory space that is used to create a new process
pub struct LinuxLoader<'a> {
    pub memory_space: MemorySpace,
    pub entry_pc: VirtualAddress,
    pub stack_top: VirtualAddress,
    pub argv_base: VirtualAddress,
    pub envp_base: VirtualAddress,
    pub ctx: ProcessContext<'a>,
    pub executable: String,
}

// Fix that `TaskControlBlock::from(memory_space_builder)` complains `Arc<MemorySpaceBuilder>` is not `Send` and `Sync`
unsafe impl Sync for LinuxLoader<'_> {}
unsafe impl Send for LinuxLoader<'_> {}

pub trait ILoadExecutable {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize, &'static str>;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        // clippy requirement
        self.len() == 0
    }
}

impl ILoadExecutable for &[u8] {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize, &'static str> {
        if offset >= self.len() {
            return Ok(0);
        }

        let end = core::cmp::min(self.len(), offset + buf.len());
        let len = end - offset;
        buf[..len].copy_from_slice(&self[offset..end]);

        Ok(len)
    }

    fn len(&self) -> usize {
        (self as &[u8]).len()
    }
}

impl ILoadExecutable for dyn IInode {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize, &'static str> {
        let this = self as &dyn IInode;

        this.readat(offset, buf).map_err(|_| "Failed to read")
    }

    fn len(&self) -> usize {
        let this = self as &dyn IInode;

        this.metadata().size
    }
}

impl ILoadExecutable for Arc<DirectoryTreeNode> {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize, &'static str> {
        let this = self as &Arc<DirectoryTreeNode>;

        this.readat(offset, buf).map_err(|_| "Failed to read")
    }

    fn len(&self) -> usize {
        let this = self as &Arc<DirectoryTreeNode>;

        this.metadata().size
    }
}

impl<'a> LinuxLoader<'a> {
    pub fn from_raw(
        data: &impl ILoadExecutable,
        path: &str,
        ctx: ProcessContext<'a>,
        auxv_values: AuxVecValues<'a>,
        fs: Arc<DirectoryTreeNode>,
        mmu: Arc<SpinMutex<dyn IMMU>>,
        alloc: Arc<SpinMutex<dyn IFrameAllocator>>,
    ) -> Result<Self, &'static str> {
        fn init<'a>(
            mut loader: LinuxLoader<'a>,
            ctx: &ProcessContext<'a>,
            auxv_values: &AuxVecValues<'a>,
        ) -> Result<LinuxLoader<'a>, &'static str> {
            loader.init_stack(ctx, auxv_values)?;

            Ok(loader)
        }

        match Self::from_shebang(data, path, fs, &mmu, &alloc) {
            Ok(shebang) => return init(shebang, &ctx, &auxv_values),
            Err(e) => {
                trace!("Failed to load shebang: {}", e.message);
            }
        };

        match LinuxLoader::from_elf(data, path, ProcessContext::default(), &mmu, &alloc) {
            Ok(elf) => return init(elf, &ctx, &auxv_values),
            Err(e) => trace!("Failed to load elf: {}", e.message),
        }

        Err("Not a valid executable")
    }

    pub fn init_stack(
        &mut self,
        ctx: &ProcessContext<'a>,
        auxv_values: &AuxVecValues<'a>,
    ) -> Result<(), &'static str> {
        self.ctx.merge(ctx, false).map_err(|e| e.into())?;
        self.ctx.auxv.insert(AuxVecKey::AT_NULL, 0);

        let mut stack_top = self.stack_top;

        let mut envps = Vec::new(); // envp pointers

        // Step1: Copy envp strings vector to the stack
        for env in self.ctx.envp.iter().rev() {
            self.push(0u8, &mut stack_top); // NULL-terminated
            for byte in env.bytes().rev() {
                self.push(byte, &mut stack_top);
            }
            envps.push(stack_top);
        }

        let mut argvs = Vec::new(); // argv pointers

        // Step2: Copy args strings vector to the stack
        for arg in self.ctx.argv.iter().rev() {
            self.push(0u8, &mut stack_top); // NULL-terminated
            for byte in arg.bytes().rev() {
                self.push(byte, &mut stack_top);
            }
            argvs.push(stack_top);
        }

        // align stack top down to 8 bytes
        stack_top = stack_top.align_down(8);
        debug_assert!(stack_top.as_usize().is_multiple_of(8));

        // Step3: Copy auxv values to stack, such as AT_RANDOM, AT_PLATFORM
        if let Some(random) = auxv_values.random {
            self.push(random, &mut stack_top);
            self.ctx
                .auxv
                .insert(AuxVecKey::AT_RANDOM, stack_top.as_usize());
        }

        if let Some(platform) = auxv_values.platform {
            let len = platform.len() + 1; // null terminated

            // Ensure that start address of copied PLATFORM is aligned to 8 bytes
            stack_top -= len;
            stack_top = stack_top.align_down(8);
            debug_assert!(stack_top.as_usize().is_multiple_of(8));
            stack_top += len;

            self.push(0, &mut stack_top); // ensure null termination

            for byte in platform.bytes().rev() {
                self.push(byte, &mut stack_top);
            }

            self.ctx
                .auxv
                .insert(AuxVecKey::AT_PLATFORM, stack_top.as_usize());
        }

        // Step4: setup aux vector

        // Collects the auxv entries in a specific order
        let auxv = self.ctx.auxv.collect();

        // Push other auxv entries
        for aux in auxv.iter() {
            self.push(aux.value, &mut stack_top);
            self.push(aux.key, &mut stack_top);
        }

        // Ensure that the last entry is AT_NULL
        debug_assert_eq!(auxv.iter().last().unwrap().key, AuxVecKey::AT_NULL);

        // Step5: setup envp vector

        // push NULL for envp
        self.push(0usize, &mut stack_top);

        // push envp, envps is already in reverse order
        for env in envps.iter() {
            self.push(*env, &mut stack_top);
        }

        let envp_base = stack_top;

        // Step6: setup argv vector

        // push NULL for args
        self.push(0usize, &mut stack_top);

        // push args, argvs is already in reverse order
        for arg in argvs.iter() {
            self.push(*arg, &mut stack_top);
        }

        let argv_base = stack_top;

        // Step7: setup argc

        // push argc
        let argc = self.ctx.argv.len();
        self.push(argc, &mut stack_top);

        self.stack_top = stack_top;
        self.argv_base = argv_base;
        self.envp_base = envp_base;

        Ok(())
    }

    fn push<T: Copy>(&self, value: T, stack_top: &mut VirtualAddress) {
        // let kernel_pt = page_table::get_kernel_page_table();

        *stack_top -= core::mem::size_of::<T>();
        *stack_top = stack_top.align_down(core::mem::align_of::<T>());

        let pt = self.memory_space.mmu().lock();

        pt.export(*stack_top, value).unwrap();
    }
}

pub struct LoadError<'a> {
    pub message: &'a str,
}

impl<'a> LoadError<'a> {
    pub fn new(message: &'a str) -> Self {
        Self { message }
    }
}

impl Debug for LoadError<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LoadError")
            .field("message", &self.message)
            .finish()
    }
}
