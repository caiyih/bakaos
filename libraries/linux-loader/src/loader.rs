use crate::{auxv::*, ProcessContext};
use abstractions::IUsizeAlias;
use address::{IAddressBase, IAlignableAddress, VirtualAddress};
use alloc::{fmt::Debug, string::String, sync::Arc, vec::Vec};
use allocation_abstractions::IFrameAllocator;
use core::ops::{Deref, DerefMut};
use filesystem_abstractions::{DirectoryTreeNode, IInode};
use hermit_sync::SpinMutex;
use memory_space::MemorySpace;
use mmu_abstractions::IMMU;
use stream::{IMMUStreamExt, MemoryStreamMut, Whence};

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

        match Self::from_shebang(data, path, fs, &mmu, &alloc) {
            Ok(shebang) => return init(shebang, &ctx, &auxv_values, cross_mmu),
            Err(e) if e.is_format_determined() => return Err(e),
            _ => (),
        };

        match LinuxLoader::from_elf(data, path, ProcessContext::default(), &mmu, &alloc) {
            Ok(elf) => return init(elf, &ctx, &auxv_values, cross_mmu),
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
        cross_mmu: Option<&Arc<SpinMutex<dyn IMMU>>>,
        ctx: &ProcessContext<'a>,
        auxv_values: &AuxVecValues<'a>,
    ) -> Result<(), LoadError> {
        self.ctx.merge(ctx, false)?;
        self.ctx.auxv.insert(AuxVecKey::AT_NULL, 0);

        let guest_mmu = self.memory_space.mmu(); // the mmu for the new process
        let target_mmu = cross_mmu.unwrap_or(guest_mmu); // the active mmu when executing this function

        let mut _target_mmu = target_mmu.lock();
        let _guest_mmu;
        let stream = match Arc::ptr_eq(target_mmu, guest_mmu) {
            true => _target_mmu.create_stream_mut(self.stack_top, false),
            false => {
                _guest_mmu = guest_mmu.lock();
                _target_mmu.create_cross_stream_mut(&*_guest_mmu, self.stack_top, false)
            }
        };

        let mut loader = StackLoader(stream);

        let mut envps = Vec::new(); // envp pointers

        // Step1: Copy envp strings vector to the stack
        for env in self.ctx.envp.iter() {
            loader.push(0u8); // NULL-terminated
            loader.push_array(env.as_bytes());
            envps.push(loader.cursor());
        }

        let mut argvs = Vec::new(); // argv pointers

        // Step2: Copy args strings vector to the stack
        for arg in self.ctx.argv.iter() {
            loader.push(0u8); // NULL-terminated
            loader.push_array(arg.as_bytes());
            argvs.push(loader.cursor());
        }

        // Step3: Copy auxv values to stack, such as AT_RANDOM, AT_PLATFORM
        if let Some(random) = auxv_values.random {
            let stack_top = loader.ensure_alignment::<usize>();
            debug_assert!(stack_top.as_usize().is_multiple_of(8));

            loader.push(random);
            self.ctx
                .auxv
                .insert(AuxVecKey::AT_RANDOM, loader.cursor().as_usize());
        }

        if let Some(platform) = auxv_values.platform {
            let len = platform.len() + 1; // null terminated

            // Ensure that start address of copied PLATFORM is aligned to 8 bytes
            loader.seek(Whence::Offset(-(len as isize)));
            loader.ensure_alignment::<u64>(); // 8 bytes alignment
            loader.seek(Whence::Offset(len as isize));

            loader.push(0u8); // ensure null termination
            loader.push_array(platform.as_bytes());

            self.ctx
                .auxv
                .insert(AuxVecKey::AT_PLATFORM, loader.cursor().as_usize());
        }

        // Step4: setup aux vector
        loader.ensure_alignment::<VirtualAddress>();

        // Collects the auxv entries in a specific order
        let auxv = self.ctx.auxv.collect();

        // Push other auxv entries
        loader.push_array(&auxv);

        // Ensure that the last entry is AT_NULL
        debug_assert_eq!(auxv.iter().last().unwrap().key, AuxVecKey::AT_NULL);

        // Step5: setup envp vector

        loader.push(VirtualAddress::null());
        loader.push_array(&envps);
        let envp_base = loader.cursor();

        // Step6: setup argv vector

        // push NULL for args
        loader.push(VirtualAddress::null());
        loader.push_array(&argvs);

        let argv_base = loader.cursor();

        // Step7: setup argc

        // push argc
        let argc = self.ctx.argv.len();
        loader.push(argc);

        self.stack_top = loader.cursor();
        self.argv_base = argv_base;
        self.envp_base = envp_base;

        Ok(())
    }
}

struct StackLoader<'a>(MemoryStreamMut<'a>);

impl StackLoader<'_> {
    /// Pushes a value onto the guest stack.
    ///
    /// Decrements `stack_top` by the size of `T` and writes `value` into
    /// the loader's memory space at the resulting address using the MMU.
    #[inline]
    pub fn push<T: Copy>(&mut self, value: T) {
        let stack_top = self.seek(Whence::Offset(-(core::mem::size_of::<T>() as isize)));

        debug_assert!(stack_top
            .as_usize()
            .is_multiple_of(core::mem::align_of::<T>()));

        *self.pwrite().unwrap() = value;
    }

    /// Pushes a slice onto the guest stack.
    ///
    /// Decrements `stack_top` by the size of `T` and writes `value` into
    /// the loader's memory space at the resulting address using the MMU.
    ///
    /// The slice is copied into the loader's memory space.
    #[inline]
    pub fn push_array<T: Copy>(&mut self, array: &[T]) {
        let stack_top = self.seek(Whence::Offset(-(core::mem::size_of_val(array) as isize)));

        debug_assert!(stack_top
            .as_usize()
            .is_multiple_of(core::mem::align_of::<T>()));
        self.pwrite_slice(array.len())
            .unwrap()
            .copy_from_slice(array);
    }

    /// Align the stack top to the given alignment.
    #[inline]
    pub fn ensure_alignment<T>(&mut self) -> VirtualAddress {
        let cursor = self.cursor().align_down(core::mem::align_of::<T>());
        self.seek(Whence::Set(cursor))
    }
}

impl<'a> Deref for StackLoader<'a> {
    type Target = MemoryStreamMut<'a>;

    fn deref(&self) -> &MemoryStreamMut<'a> {
        &self.0
    }
}

impl<'a> DerefMut for StackLoader<'a> {
    fn deref_mut(&mut self) -> &mut MemoryStreamMut<'a> {
        &mut self.0
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

#[cfg(test)]
mod tests {
    use address::{IToPageNum, VirtualPageNumRange};
    use memory_space::MappingArea;
    use mmu_abstractions::{GenericMappingFlags, PageSize};
    use stream::MemoryStream;
    use test_utilities::allocation::contiguous::TestFrameAllocator;

    use super::*;

    fn test_scene(ctx: ProcessContext<'_>, action: impl FnOnce(LinuxLoader<'_>)) {
        let (alloc, mmu) = TestFrameAllocator::new_with_mmu(1024 * 1024 * 1024);

        let mut memory_space = MemorySpace::new(mmu, alloc);

        // Allocate 2 MB for the stack.
        let stack_base = VirtualAddress::from_usize(0x80000000);
        let stack_size = PageSize::_2M.as_usize();

        memory_space.alloc_and_map_area(MappingArea {
            range: VirtualPageNumRange::from_start_count(
                stack_base.to_floor_page_num(),
                stack_size / PageSize::_4K.as_usize(),
            ),
            area_type: memory_space::AreaType::UserStack,
            map_type: memory_space::MapType::Framed,
            permissions: GenericMappingFlags::User
                | GenericMappingFlags::Kernel
                | GenericMappingFlags::Readable
                | GenericMappingFlags::Writable,
            allocation: None,
        });

        let loader = LinuxLoader {
            memory_space,
            entry_pc: VirtualAddress::from_usize(0x10000),
            stack_top: stack_base + stack_size,
            argv_base: VirtualAddress::null(),
            envp_base: VirtualAddress::null(),
            ctx,
            executable: String::new(),
        };

        action(loader);
    }

    #[test]
    fn test_stack_alignment() {
        test_scene(ProcessContext::default(), |mut loader| {
            let mut ctx = ProcessContext::new();
            ctx.extend_argv(&[alloc::borrow::Cow::Borrowed("test")])
                .unwrap();

            let auxv_values = AuxVecValues {
                random: Some([0u8; 16]),
                platform: Some("test_platform"),
            };

            loader.init_stack(None, &ctx, &auxv_values).unwrap();

            // Stack top should be aligned to 8
            assert_eq!(
                loader.stack_top.as_usize() % 8,
                0,
                "Stack top should be 8-byte aligned"
            );

            // Verify pointers alignment
            assert_eq!(
                loader.argv_base.as_usize() % 8,
                0,
                "argv_base should be 8-byte aligned"
            );
            assert_eq!(
                loader.envp_base.as_usize() % 8,
                0,
                "envp_base should be 8-byte aligned"
            );
        });
    }

    #[test]
    fn test_stack_layout_minimal() {
        // Test minimal stack layout
        test_scene(ProcessContext::default(), |mut loader| {
            let ctx = ProcessContext::new();
            let auxv_values = AuxVecValues::default();

            loader.init_stack(None, &ctx, &auxv_values).unwrap();

            let mmu = loader.memory_space.mmu().lock();
            let mut stream = mmu.create_stream(loader.stack_top, false);

            let argc: usize = *stream.read().unwrap();
            assert_eq!(argc, 0);

            let argv_null: VirtualAddress = *stream.read().unwrap();
            assert!(argv_null.is_null());

            let envp_null: VirtualAddress = *stream.read().unwrap();
            assert!(envp_null.is_null());

            let auxv_key: AuxVecKey = *stream.read().unwrap();
            let auxv_value: usize = *stream.read().unwrap();
            assert_eq!(auxv_key, AuxVecKey::AT_NULL);
            assert_eq!(auxv_value, 0);
        });
    }

    #[test]
    fn test_stack_layout() {
        use crate::auxv::{AuxVecKey, AuxVecValues};
        use alloc::borrow::Cow;

        test_scene(ProcessContext::default(), |mut loader| {
            let mut ctx = ProcessContext::new();

            ctx.extend_argv(&[
                Cow::Borrowed("./test_program"),
                Cow::Borrowed("arg1"),
                Cow::Borrowed("arg2"),
                Cow::Borrowed("hello world"),
            ])
            .unwrap();

            ctx.extend_envp(&[
                Cow::Borrowed("PATH=/usr/bin:/bin"),
                Cow::Borrowed("HOME=/home/user"),
                Cow::Borrowed("SHELL=/bin/bash"),
                Cow::Borrowed("TEST_VAR=test_value"),
            ])
            .unwrap();

            ctx.auxv.insert(AuxVecKey::AT_ENTRY, 0x400000);
            ctx.auxv.insert(AuxVecKey::AT_PHDR, 0x400040);
            ctx.auxv.insert(AuxVecKey::AT_PHENT, 56);
            ctx.auxv.insert(AuxVecKey::AT_PHNUM, 9);
            ctx.auxv.insert(AuxVecKey::AT_PAGESZ, 4096);
            ctx.auxv.insert(AuxVecKey::AT_BASE, 0x7f0000000000);
            ctx.auxv.insert(AuxVecKey::AT_FLAGS, 0);
            ctx.auxv.insert(AuxVecKey::AT_UID, 1000);
            ctx.auxv.insert(AuxVecKey::AT_EUID, 1000);
            ctx.auxv.insert(AuxVecKey::AT_GID, 1000);
            ctx.auxv.insert(AuxVecKey::AT_EGID, 1000);
            ctx.auxv.insert(AuxVecKey::AT_SECURE, 0);
            ctx.auxv.insert(AuxVecKey::AT_CLKTCK, 100);
            ctx.auxv.insert(AuxVecKey::AT_HWCAP, 0x178bfbff);

            let random_bytes = [
                0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54,
                0x32, 0x10,
            ];
            let auxv_values = AuxVecValues {
                random: Some(random_bytes),
                platform: Some("x86_64"),
            };

            loader.init_stack(None, &ctx, &auxv_values).unwrap();

            verify_stack_layout(&loader, &ctx, &auxv_values);
        });
    }

    // Verify that the stack layout matches the expected structure
    fn verify_stack_layout(loader: &LinuxLoader, ctx: &ProcessContext, auxv_values: &AuxVecValues) {
        let mmu = loader.memory_space.mmu().lock();
        let mut stream = mmu.create_stream(loader.stack_top, false);

        let argc: usize = *stream.read().unwrap();
        assert_eq!(
            argc,
            ctx.argv.len(),
            "argc should match the number of arguments"
        );

        let mut argv_pointers = Vec::new();
        for i in 0..argc {
            let ptr: VirtualAddress = *stream.read().unwrap();
            argv_pointers.push(ptr);
            assert!(!ptr.is_null(), "argv[{}] should not be null", i);
        }

        let argv_null: VirtualAddress = *stream.read().unwrap();
        assert!(argv_null.is_null(), "argv array should be null-terminated");

        let envp_pointers = stream
            .read_unsized_slice::<VirtualAddress>(|ptr, _| !ptr.is_null())
            .unwrap()
            .to_vec();

        assert!(
            stream.read::<VirtualAddress>().unwrap().is_null(),
            "envp array should be null-terminated"
        );

        assert_eq!(
            envp_pointers.len(),
            ctx.envp.len(),
            "envp count should match"
        );

        let mut auxv_entries = Vec::new();
        loop {
            let key: AuxVecKey = *stream.read().unwrap();
            let value: usize = *stream.read().unwrap();

            auxv_entries.push((key, value));
            if key == AuxVecKey::AT_NULL {
                break;
            }
        }

        verify_common_auxv_entries(&auxv_entries, ctx);

        verify_string_array_contents(&mut stream, &argv_pointers, &ctx.argv);
        verify_string_array_contents(&mut stream, &envp_pointers, &ctx.envp);

        verify_special_auxv_values(&mut stream, &auxv_entries, auxv_values);
    }

    fn verify_common_auxv_entries(auxv_entries: &[(AuxVecKey, usize)], _ctx: &ProcessContext) {
        assert_eq!(auxv_entries.last().unwrap().0, AuxVecKey::AT_NULL);

        let auxv_map: alloc::collections::BTreeMap<AuxVecKey, usize> =
            auxv_entries.iter().copied().collect();

        assert_eq!(auxv_map.get(&AuxVecKey::AT_ENTRY), Some(&0x400000));
        assert_eq!(auxv_map.get(&AuxVecKey::AT_PHDR), Some(&0x400040));
        assert_eq!(auxv_map.get(&AuxVecKey::AT_PAGESZ), Some(&4096));
        assert_eq!(auxv_map.get(&AuxVecKey::AT_UID), Some(&1000));
        assert_eq!(auxv_map.get(&AuxVecKey::AT_CLKTCK), Some(&100));
    }

    fn verify_string_array_contents(
        stream: &mut MemoryStream,
        pointers: &[VirtualAddress],
        expected_strings: &[alloc::borrow::Cow<str>],
    ) {
        for (i, (ptr, expected)) in pointers.iter().zip(expected_strings.iter()).enumerate() {
            stream.seek(Whence::Set(*ptr));
            let bytes = stream.read_unsized_slice::<u8>(|&c, _| c != b'\0').unwrap();

            let actual_string = core::str::from_utf8(bytes).unwrap();
            assert_eq!(
                actual_string,
                expected.as_ref(),
                "String at index {} should match",
                i
            );
        }
    }

    fn verify_special_auxv_values(
        stream: &mut MemoryStream,
        auxv_entries: &[(AuxVecKey, usize)],
        auxv_values: &AuxVecValues,
    ) {
        let auxv_map: alloc::collections::BTreeMap<AuxVecKey, usize> =
            auxv_entries.iter().copied().collect();

        if let Some(expected_random) = auxv_values.random {
            let random = auxv_map.get(&AuxVecKey::AT_RANDOM).unwrap();
            stream.seek(Whence::Set(VirtualAddress::from_usize(*random)));

            let actual_random = *stream.read::<[u8; 16]>().unwrap();

            assert_eq!(actual_random, expected_random);
        }

        if let Some(expected_platform) = auxv_values.platform {
            let platform_addr = auxv_map.get(&AuxVecKey::AT_PLATFORM).unwrap();
            stream.seek(Whence::Set(VirtualAddress::from_usize(*platform_addr)));

            let actual_platform = stream.read_unsized_slice::<u8>(|&c, _| c != b'\0').unwrap();
            let actual_platform = String::from_utf8_lossy(actual_platform);
            assert_eq!(actual_platform, expected_platform,);
        }
    }
}
