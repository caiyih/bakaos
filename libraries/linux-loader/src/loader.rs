use crate::{auxv::*, ProcessContext, RawMemorySpace};
use address::VirtualAddress;
use alloc::{fmt::Debug, string::String, sync::Arc};
use filesystem_abstractions::{DirectoryTreeNode, IInode};
use hermit_sync::SpinMutex;
use memory_space::MemorySpace;
use mmu_abstractions::IMMU;

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
        memory_space: &RawMemorySpace,
        cross_mmu: Option<&Arc<SpinMutex<dyn IMMU>>>,
    ) -> Result<Self, LoadError> {
        fn init<'a>(
            mut loader: LinuxLoader<'a>,
            ctx: &ProcessContext<'a>,
            auxv_values: &AuxVecValues<'a>,
            cross_mmu: Option<&Arc<SpinMutex<dyn IMMU>>>,
        ) -> Result<LinuxLoader<'a>, LoadError> {
            loader.init_stack(cross_mmu, ctx, auxv_values)?;

            Ok(loader)
        }

        // Try loading as shebang first
        match Self::from_shebang(data, path, fs.clone(), memory_space) {
            Ok(shebang) => return init(shebang, &ctx, &auxv_values, cross_mmu),
            Err(e) if e.is_format_determined() => return Err(e),
            Err(_) => (), // Continue to try ELF
        }

        // If shebang didn't work, try ELF
        match Self::from_elf(data, path, ProcessContext::default(), memory_space) {
            Ok(elf) => init(elf, &ctx, &auxv_values, cross_mmu),
            Err(e) if e.is_format_determined() => Err(e),
            Err(_) => Err(LoadError::NotExecutable),
        }
    }
}

/// The error type for the `LinuxLoader`'s `load` methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadError {
    /// The given file can not be parsed as an executable.
    NotExecutable,
    /// The executable is not compatible with the current operating system.
    OsMismatch,
    /// The executable is not compatible with the current architecture.
    ArchMismatch,
    /// The kernel ran out of memory.
    InsufficientMemory,
    /// Error occurred while reading the executable.
    UnableToReadExecutable,
    /// Memory management units failed to load the executable.
    FailedToLoad,
    /// The executable is incomplete.
    IncompleteExecutable,
    /// The executable is too large.
    TooLarge,
    /// The executable requires an interpreter, but it can not be found.
    CanNotFindInterpreter,
    /// The shebang string is invalid.
    InvalidShebangString,
    /// The executable is not a valid ELF executable.
    NotElf,
    /// The executable is not a valid shebang executable.
    NotShebang,
    /// The required argument count is exceeded.
    ArgumentCountExceeded,
    /// The required environment variable count is exceeded.
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
