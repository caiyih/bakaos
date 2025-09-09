use abstractions::IUsizeAlias;
use address::{IAlignableAddress, VirtualAddress};
use alloc::{fmt::Debug, string::String, sync::Arc, vec::Vec};
use allocation_abstractions::IFrameAllocator;
use filesystem_abstractions::{DirectoryTreeNode, IInode};
use hermit_sync::SpinMutex;
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

/// Represent a random-readable executable file source
pub trait IExecSource {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize, &'static str>;

    fn len(&self) -> usize;

    /// Returns true if the executable source has zero length.
    ///
    /// This is the default implementation of `is_empty` for `IExecSource` and
    /// simply checks whether `len()` is 0.
    ///
    /// # Examples
    ///
    /// ```
    /// // For an implementor, the default can be used:
    /// use linux_loader::IExecSource;
    ///
    /// struct EmptySource;
    ///
    /// impl IExecSource for EmptySource {
    ///     fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> Result<usize, &'static str> { Ok(0) }
    ///     fn len(&self) -> usize { 0 }
    /// }
    /// let src = EmptySource;
    /// assert!(src.is_empty());
    /// ```
    fn is_empty(&self) -> bool {
        // clippy requirement
        self.len() == 0
    }
}

impl IExecSource for &[u8] {
    /// Reads up to `buf.len()` bytes from this byte slice starting at `offset` into `buf`.
    ///
    /// Returns the number of bytes copied. If `offset` is past the end of the slice, returns `Ok(0)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use linux_loader::IExecSource;
    ///
    /// let data: &[u8] = b"hello";
    /// let mut buf = [0u8; 3];
    /// let n = data.read_at(1, &mut buf).unwrap();
    /// assert_eq!(n, 3);
    /// assert_eq!(&buf, b"ell");
    /// ```
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize, &'static str> {
        if offset >= self.len() {
            return Ok(0);
        }

        let end = core::cmp::min(self.len(), offset + buf.len());
        let len = end - offset;
        buf[..len].copy_from_slice(&self[offset..end]);

        Ok(len)
    }

    /// Returns the length in bytes of the underlying byte slice.
    ///
    /// # Examples
    ///
    /// ```
    /// let data: &[u8] = b"hello";
    /// assert_eq!(data.len(), data.len());
    /// ```
    fn len(&self) -> usize {
        (self as &[u8]).len()
    }
}

impl IExecSource for dyn IInode {
    /// Read up to `buf.len()` bytes from the inode at `offset`.
    ///
    /// Delegates to the inode's `readat` method and maps any read error to the static
    /// string `"Failed to read"`. Returns the number of bytes actually read on success.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// # // pseudo-code: `inode` must implement `IInode` and be in scope as a trait object
    /// # let inode: &dyn IInode = /* ... */;
    /// let mut buf = [0u8; 16];
    /// let n = inode.read_at(0, &mut buf).expect("read failed");
    /// assert!(n <= buf.len());
    /// ```
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize, &'static str> {
        let this = self as &dyn IInode;

        this.readat(offset, buf).map_err(|_| "Failed to read")
    }

    /// Returns the total size (in bytes) of the underlying inode.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use linux_loader::IExecSource;
    ///
    /// // Given an object `inode` that implements `IInode`:
    /// let size = (inode as &dyn IExecSource).len();
    /// ```
    fn len(&self) -> usize {
        let this = self as &dyn IInode;

        this.metadata().size
    }
}

impl IExecSource for Arc<DirectoryTreeNode> {
    /// Reads up to `buf.len()` bytes from this directory node starting at `offset` into `buf`.
    ///
    /// Returns the number of bytes actually read on success. Any underlying read error is
    /// mapped to a static `Err("Failed to read")`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use linux_loader::IExecSource;
    ///
    /// // Assume `node` is an `Arc<DirectoryTreeNode>` previously opened and populated.
    /// let mut buf = [0u8; 16];
    /// let n = node.read_at(0, &mut buf).expect("read failed");
    /// assert!(n <= buf.len());
    /// ```
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize, &'static str> {
        let this = self as &Arc<DirectoryTreeNode>;

        this.readat(offset, buf).map_err(|_| "Failed to read")
    }

    /// Returns the total size (in bytes) of the underlying directory-tree node.
    ///
    /// This is the length used by IExecSource to represent how many bytes the
    /// executable source contains.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // `node` is an `Arc<DirectoryTreeNode>`
    /// let size = node.len();
    /// assert_eq!(size, node.metadata().size);
    /// ```
    fn len(&self) -> usize {
        let this = self as &Arc<DirectoryTreeNode>;

        this.metadata().size
    }
}

impl<'a> LinuxLoader<'a> {
    /// Attempts to create a LinuxLoader from raw executable data, trying known formats in order (shebang, then ELF),
    /// and initializes the user stack with the provided ProcessContext and auxiliary values on success.
    ///
    /// This function:
    /// - Tries to interpret `data` as a shebang script; if successful, constructs the loader and calls `init_stack`.
    /// - If the shebang attempt fails but determines the format conclusively, the error is returned.
    /// - Otherwise, tries to load as an ELF image (using a default ProcessContext for format detection); on success the
    ///   loader is initialized with the provided `ctx` and `auxv_values`.
    /// - If neither loader succeeds, returns `LoadError::NotExecutable`.
    ///
    /// The returned LinuxLoader has its memory space prepared and its stack initialized (argv, envp, auxv, argc),
    /// with `argv_base` and `envp_base` set for later use.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// // Assume `buf`, `path`, `ctx`, `auxv_values`, `fs`, `mmu`, `alloc` are available in the calling context.
    /// // let loader = LinuxLoader::from_raw(&buf, &path, ctx, auxv_values, fs, mmu, alloc)?;
    /// ```
    pub fn from_raw(
        data: &impl IExecSource,
        path: &str,
        ctx: ProcessContext<'a>,
        auxv_values: AuxVecValues<'a>,
        fs: Arc<DirectoryTreeNode>,
        mmu: Arc<SpinMutex<dyn IMMU>>,
        alloc: Arc<SpinMutex<dyn IFrameAllocator>>,
    ) -> Result<Self, LoadError> {
        fn init<'a>(
            mut loader: LinuxLoader<'a>,
            ctx: &ProcessContext<'a>,
            auxv_values: &AuxVecValues<'a>,
        ) -> Result<LinuxLoader<'a>, LoadError> {
            loader.init_stack(ctx, auxv_values)?;

            Ok(loader)
        }

        match Self::from_shebang(data, path, fs, &mmu, &alloc) {
            Ok(shebang) => return init(shebang, &ctx, &auxv_values),
            Err(e) if e.is_format_determined() => return Err(e),
            _ => (),
        };

        match LinuxLoader::from_elf(data, path, ProcessContext::default(), &mmu, &alloc) {
            Ok(elf) => return init(elf, &ctx, &auxv_values),
            Err(e) if e.is_format_determined() => return Err(e),
            _ => (),
        }

        Err(LoadError::NotExecutable)
    }

    /// Initialize the initial user stack layout (strings, pointers, auxv, and argc) for the loader's memory space.
    ///
    /// This writes environment strings (envp) and program arguments (argv) into guest memory, places auxiliary
    /// vector entries (including AT_RANDOM and AT_PLATFORM when provided), builds the envp/argv pointer arrays,
    /// and finally writes argc. After success the loader's `stack_top`, `argv_base`, and `envp_base` are updated
    /// to reflect the constructed stack layout and `self.ctx` is merged with the provided `ctx`.
    ///
    /// Returns `Err(LoadError)` if merging the context or any memory writes required to build the stack fail.
    pub fn init_stack(
        &mut self,
        ctx: &ProcessContext<'a>,
        auxv_values: &AuxVecValues<'a>,
    ) -> Result<(), LoadError> {
        self.ctx.merge(ctx, false)?;
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

    /// Pushes a value onto the guest stack.
    ///
    /// Decrements `stack_top` by the size of `T`, aligns it down to `T`'s alignment, and writes `value` into
    /// the loader's memory space at the resulting address using the MMU. The provided `stack_top` is updated
    /// in place to the new top-of-stack address.
    ///
    /// # Examples
    ///
    /// ```
    /// // Prepare a loader and stack_top, then push a 64-bit value:
    /// // let mut loader = /* LinuxLoader with initialized memory_space and mmu */;
    /// // let mut stack_top = loader.stack_top;
    /// // loader.push(0u64, &mut stack_top);
    /// ```
    fn push<T: Copy>(&self, value: T, stack_top: &mut VirtualAddress) {
        // let kernel_pt = page_table::get_kernel_page_table();

        *stack_top -= core::mem::size_of::<T>();
        *stack_top = stack_top.align_down(core::mem::align_of::<T>());

        let pt = self.memory_space.mmu().lock();

        pt.export(*stack_top, value).unwrap();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadError {
    NotExecutable,
    OsMismatch,
    ArchMismatch,
    InsufficientMemory,
    UnableToReadExecutable,
    FailedToLoad,
    IncompleteExecutable,
    TooLarge,
    CanNotFindInterpreter,
    InvalidShebangString,
    NotElf,
    NotShebang,
    ArgumentCountExceeded,
    EnvironmentCountExceeded,
}

impl LoadError {
    /// Returns whether this `LoadError` conclusively determines the executable format.
    ///
    /// Some errors indicate a definite determination about the executable's format (e.g. `NotExecutable`,
    /// architecture/OS mismatches, truncated/invalid binaries), while others mean the loader could not
    /// read enough data to decide (e.g. `UnableToReadExecutable`, `NotElf`, `NotShebang`).
    ///
    /// # Examples
    ///
    /// ```
    /// use linux_loader::LoadError;
    ///
    /// assert!(LoadError::NotExecutable.is_format_determined());
    /// assert!(!LoadError::NotElf.is_format_determined());
    /// ```
    pub fn is_format_determined(&self) -> bool {
        match *self {
            LoadError::NotExecutable
            | LoadError::OsMismatch
            | LoadError::ArchMismatch
            | LoadError::InsufficientMemory
            | LoadError::FailedToLoad
            | LoadError::IncompleteExecutable
            | LoadError::TooLarge
            | LoadError::CanNotFindInterpreter
            | LoadError::InvalidShebangString
            | LoadError::ArgumentCountExceeded
            | LoadError::EnvironmentCountExceeded => true,
            LoadError::UnableToReadExecutable | LoadError::NotElf | LoadError::NotShebang => false,
        }
    }
}
