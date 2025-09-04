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

pub trait ILoadExecutable {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize, &'static str>;

    fn len(&self) -> usize;

    /// Default implementation that reports whether the executable source has zero length.
    ///
    /// Returns true when `len()` is 0, false otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// // Call the trait method via fully qualified syntax on a byte slice (which implements `ILoadExecutable`).
    /// let data: &[u8] = b"";
    /// assert!(ILoadExecutable::is_empty(&data));
    /// let nonempty: &[u8] = b"abc";
    /// assert!(!ILoadExecutable::is_empty(&nonempty));
    /// ```
    fn is_empty(&self) -> bool {
        // clippy requirement
        self.len() == 0
    }
}

impl ILoadExecutable for &[u8] {
    /// Reads bytes from the slice into `buf` starting at `offset`.
    ///
    /// If `offset` is past the end of the slice this returns `Ok(0)`. Otherwise it copies
    /// up to `buf.len()` bytes (or fewer if the slice ends sooner) into `buf` and returns
    /// the number of bytes written. This method does not return errors.
    ///
    /// # Examples
    ///
    /// ```
    /// let data: &[u8] = b"hello";
    /// let mut buf = [0u8; 4];
    /// let n = data.read_at(1, &mut buf).unwrap();
    /// assert_eq!(n, 4);
    /// assert_eq!(&buf, b"ello");
    ///
    /// let mut buf2 = [0u8; 3];
    /// let n2 = data.read_at(5, &mut buf2).unwrap(); // offset at end
    /// assert_eq!(n2, 0);
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

    /// Returns the number of bytes in the underlying byte slice.
    ///
    /// # Examples
    ///
    /// ```
    /// let s: &[u8] = b"hello";
    /// assert_eq!(ILoadExecutable::len(&s), 5);
    /// ```
    fn len(&self) -> usize {
        (self as &[u8]).len()
    }
}

impl ILoadExecutable for dyn IInode {
    /// Read bytes from the underlying inode into `buf` starting at `offset`.
    ///
    /// Returns the number of bytes read on success or `Err("Failed to read")` if the underlying
    /// inode read operation fails. The error is a static string to match the `ILoadExecutable`
    /// trait's error type.
    ///
    /// # Examples
    ///
    /// ```
    /// // `node` implements `IInode` (e.g., obtained from a filesystem).
    /// let node: &dyn IInode = /* ... */;
    /// let mut buf = [0u8; 128];
    /// let n = node.read_at(0, &mut buf).expect("read failed");
    /// assert!(n <= buf.len());
    /// ```
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize, &'static str> {
        let this = self as &dyn IInode;

        this.readat(offset, buf).map_err(|_| "Failed to read")
    }

    /// Returns the inode's stored size in bytes (metadata.size).
    ///
    /// This delegates to the underlying `IInode` implementation's metadata and returns its
    /// `size` field.
    ///
    /// # Examples
    ///
    /// ```
    /// struct MockInode { meta_size: usize }
    /// impl IInode for MockInode {
    ///     fn metadata(&self) -> Metadata { Metadata { size: self.meta_size, ..Default::default() } }
    ///     // other trait methods omitted for brevity...
    /// }
    ///
    /// let inode = MockInode { meta_size: 42 };
    /// let sized = (&inode as &dyn IInode).metadata().size;
    /// assert_eq!(sized, 42);
    /// ```
    fn len(&self) -> usize {
        let this = self as &dyn IInode;

        this.metadata().size
    }
}

impl ILoadExecutable for Arc<DirectoryTreeNode> {
    /// Read bytes from the underlying directory-tree node at the given offset.
    ///
    /// Delegates to `DirectoryTreeNode::readat`, returning the number of bytes
    /// successfully read or `Err("Failed to read")` if the underlying read fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// // `node` is an existing Arc<DirectoryTreeNode>.
    /// let node: Arc<DirectoryTreeNode> = unimplemented!();
    /// let mut buf = [0u8; 128];
    /// let bytes = node.read_at(0, &mut buf).expect("read failed");
    /// println!("read {} bytes", bytes);
    /// ```
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize, &'static str> {
        let this = self as &Arc<DirectoryTreeNode>;

        this.readat(offset, buf).map_err(|_| "Failed to read")
    }

    /// Returns the size, in bytes, of the file represented by this DirectoryTreeNode.
    ///
    /// This delegates to the node's metadata and returns its `size` field.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // `node` is an `Arc<DirectoryTreeNode>`
    /// let size = node.len();
    /// ```
    fn len(&self) -> usize {
        let this = self as &Arc<DirectoryTreeNode>;

        this.metadata().size
    }
}

impl<'a> LinuxLoader<'a> {
    /// Create a LinuxLoader by detecting and preparing an executable image from a raw source.
    ///
    /// This tries to interpret `data` first as a shebang script (interpreter directive) and, if that
    /// fails without determining a final format, as an ELF binary. On success the returned
    /// `LinuxLoader` has its stack initialized (via `init_stack`) with `ctx` merged and `auxv_values`
    /// applied. If neither shebang nor ELF parsing yields a determined executable format, returns
    /// `LoadError::NotExecutable`.
    ///
    /// Errors returned from shebang/ELF parsing that indicate a definitive format or fatal problem
    /// (for example architecture/os mismatch, insufficient memory, missing interpreter, or other
    /// errors for which `LoadError::is_format_determined()` is true) are propagated immediately.
    ///
    /// Parameters whose meaning is obvious from their names or types (e.g., filesystem, MMU, frame
    /// allocator) are not documented here; `data` is any implementation of `ILoadExecutable` providing
    /// read-at/length access to the raw executable bytes, `path` is the filesystem path used for
    /// resolution/context, `ctx` is the process context to merge into the loader, and `auxv_values`
    /// supplies auxiliary vectors such as AT_RANDOM or AT_PLATFORM.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Example (pseudocode): load from a byte slice, then inspect entry point.
    /// let bytes: &[u8] = include_bytes!("my_program");
    /// let ctx = ProcessContext::default();
    /// let auxv = AuxVecValues::default();
    /// let fs = Arc::new(DirectoryTreeNode::root());
    /// let mmu = /* platform MMU stub */;
    /// let alloc = /* frame allocator stub */;
    ///
    /// let loader = LinuxLoader::from_raw(bytes, "/bin/my_program", ctx, auxv, fs, mmu, alloc)
    ///     .expect("executable must be loadable");
    /// println!("entry PC = {:#x}", loader.entry_pc);
    /// ```
    pub fn from_raw(
        data: &impl ILoadExecutable,
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

    /// Initialize the process stack for the loader: copies argv and envp strings and pointers,
    /// places auxiliary vector entries (including AT_RANDOM and AT_PLATFORM when provided),
    /// aligns the stack, and writes argc, argv pointers, and envp pointers into memory.
    ///
    /// This merges the provided `ctx` into the loader's context, ensures an `AT_NULL` auxv entry,
    /// and updates these loader fields on success:
    /// - `self.stack_top` — new top of stack after data is pushed
    /// - `self.argv_base` — address of the argv pointer array on the stack
    /// - `self.envp_base` — address of the envp pointer array on the stack
    ///
    /// The function may return a `LoadError` (for example if merging contexts fails).
    ///
    /// Parameters:
    /// - `ctx`: supplemental ProcessContext to merge into the loader before layout.
    /// - `auxv_values`: optional auxv payloads (e.g., AT_RANDOM and AT_PLATFORM) to be copied onto the stack.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Prepare loader, context and auxv_values appropriately, then:
    /// // loader.init_stack(&ctx, &auxv_values)?;
    /// ```
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

    /// Write a value onto the stack by allocating space below `stack_top`, aligning the
    /// address for `T`, and copying `value` into the process memory via the loader's MMU.
    ///
    /// The function decrements `stack_top` by `size_of::<T>()`, aligns the new top down
    /// to `align_of::<T>()`, and writes `value` at that address. `stack_top` is updated
    /// in place to point to the newly reserved slot.
    ///
    /// # Panics
    ///
    /// Panics if the underlying MMU `export` fails when writing `value`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Reserve space for a u64 on the stack and write 0xdeadbeef.
    /// let loader: LinuxLoader = /* obtained from loader construction */ unimplemented!();
    /// let mut stack_top: VirtualAddress = 0x7fff_ffff;
    /// loader.push(0xdead_beef_u64, &mut stack_top);
    /// // stack_top now points to the allocated u64 slot.
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
    /// Returns whether the loader error indicates the executable format was conclusively determined.
    ///
    /// When true, the error means the loader can stop trying other format parsers (e.g., shebang or ELF)
    /// because the failure is final (OS/arch mismatch, resource limits, malformed interpreter, etc.).
    /// When false, the error is transient/ambiguous with respect to format detection (e.g., I/O
    /// failure or this input is simply not an ELF or shebang), allowing other format attempts to proceed.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::loader::LoadError;
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
