use abstractions::IUsizeAlias;
use address::{
    IAddressBase, IAlignableAddress, IPageNum, IToPageNum, VirtualAddress, VirtualAddressRange,
    VirtualPageNum, VirtualPageNumRange,
};
use alloc::{string::String, sync::Arc, vec::Vec};
use allocation_abstractions::IFrameAllocator;
use filesystem_abstractions::{DirectoryTreeNode, IInode};
use hermit_sync::SpinMutex;
use log::{debug, warn};
use memory_space::{AreaType, MapType, MappingArea, MemorySpace, MemorySpaceAttribute};
use mmu_abstractions::{GenericMappingFlags, IMMU};
use utilities::InvokeOnDrop;
use xmas_elf::ElfFile;

use crate::auxv::*;

// A data structure to build a memory space that is used to create a new process
pub struct LinuxLoader {
    pub memory_space: MemorySpace,
    pub entry_pc: VirtualAddress,
    pub stack_top: VirtualAddress,
    pub argc: usize,
    pub argv_base: VirtualAddress,
    pub envp_base: VirtualAddress,
    pub auxv: Vec<AuxVecEntry>,
    pub executable: String,
    pub command_line: Vec<String>,
}

// Fix that `TaskControlBlock::from(memory_space_builder)` complains `Arc<MemorySpaceBuilder>` is not `Send` and `Sync`
unsafe impl Sync for LinuxLoader {}
unsafe impl Send for LinuxLoader {}

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
        buf[..end].copy_from_slice(&self[offset..end]);

        Ok(end - offset)
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

impl LinuxLoader {
    pub fn from_raw(
        data: &impl ILoadExecutable,
        path: &str,
        args: &[&str],
        envp: &[&str],
        fs: Arc<DirectoryTreeNode>,
        pt: Arc<SpinMutex<dyn IMMU>>,
        allocator: Arc<SpinMutex<dyn IFrameAllocator>>,
    ) -> Result<Self, &'static str> {
        if let Ok((mut shebang, shebang_args)) = Self::from_shebang(data, path, fs, &pt, &allocator)
        {
            let args_boxed = {
                let shebang_args = shebang_args
                    .split(' ')
                    .skip_while(|s| s.is_empty())
                    .collect::<Vec<_>>();

                let mut aggregate_args = Vec::with_capacity(shebang_args.len() + args.len());
                aggregate_args.extend_from_slice(&shebang_args);
                aggregate_args.extend_from_slice(args);

                aggregate_args
            };

            shebang.init_stack(&args_boxed, envp);
            Ok(shebang)
        } else if let Ok(mut elf) = Self::from_elf(data, path, &pt, &allocator) {
            elf.init_stack(args, envp);
            Ok(elf)
        } else {
            Err("Not a valid executable")
        }
    }

    fn from_shebang(
        data: &impl ILoadExecutable,
        path: &str,
        fs: Arc<DirectoryTreeNode>,
        pt: &Arc<SpinMutex<dyn IMMU>>,
        allocator: &Arc<SpinMutex<dyn IFrameAllocator>>,
    ) -> Result<(Self, String), &'static str> {
        const SHEBANG_MAX_LEN: usize = 127;
        const DEFAULT_SHEBANG: &[(&str, &[u8])] = &[(".sh", b"/bin/busybox sh")];

        let mut header = [0u8; SHEBANG_MAX_LEN + 2];

        let len = data.read_at(0, &mut header)?;

        let header = &header[..len]; // extract valid part

        // Check if the file starts with a shebang or if the path matches any default shebang pattern
        let is_shebang = len >= 3 && &header[..3] == b"#!/";
        let matches_default_shebang = DEFAULT_SHEBANG.iter().any(|f| path.ends_with(f.0));

        if !is_shebang && !matches_default_shebang {
            // Try to use the default shebang
            return Err("No interpreter specified and no default shebang found");
        }

        fn try_shebang(
            shebang: &[u8],
            fs: &Arc<DirectoryTreeNode>,
            pt: &Arc<SpinMutex<dyn IMMU>>,
            allocator: &Arc<SpinMutex<dyn IFrameAllocator>>,
        ) -> Result<(LinuxLoader, String), &'static str> {
            let shebang = core::str::from_utf8(shebang).map_err(|_| "Not a valid UTF-8 string")?;

            let (path, args) = shebang.split_once(' ').unwrap_or((shebang, ""));

            let interpreter = fs.open(path, None).map_err(|_| "Interpreter not found")?;

            Ok((
                LinuxLoader::from_elf(&interpreter, path, pt, allocator)?,
                String::from(args),
            ))
        }

        // Prefer default shebang
        for &(suffix, interpreter) in DEFAULT_SHEBANG {
            if path.ends_with(suffix) {
                if let Ok(ret) = try_shebang(interpreter, &fs, pt, allocator) {
                    return Ok(ret);
                }
            }
        }

        if !is_shebang {
            return Err("No interpreter specified");
        }

        let first_new_line = match header.iter().position(|b| *b == b'\n') {
            Some(idx) => idx,
            None => return Err("Can not find the end of the shebang within the first 128 bytes"),
        };

        debug_assert!(first_new_line > 2);

        // If the file starts with a shebang, process it
        let shebang = header[2..first_new_line].trim_ascii();

        try_shebang(shebang, &fs, pt, allocator)
    }

    pub fn from_elf(
        elf_data: &impl ILoadExecutable,
        executable_path: &str,
        pt: &Arc<SpinMutex<dyn IMMU>>,
        allocator: &Arc<SpinMutex<dyn IFrameAllocator>>,
    ) -> Result<Self, &'static str> {
        let mut memory_space = MemorySpace::new(pt.clone(), allocator.clone());
        let mut attr = MemorySpaceAttribute::default();

        // see https://github.com/caiyih/bakaos/issues/26
        let boxed_elf_holding;

        let boxed_elf;

        let elf_info = {
            let required_frames = elf_data.len().div_ceil(constants::PAGE_SIZE);

            let frames = allocator
                .lock()
                .alloc_contiguous(required_frames)
                .ok_or("Out of memory")?;

            boxed_elf_holding =
                InvokeOnDrop::transform(frames, |f| allocator.lock().dealloc_range(f));

            let pt = pt.lock();

            let slice = pt
                .translate_phys(
                    boxed_elf_holding.start,
                    boxed_elf_holding.end.as_usize() - boxed_elf_holding.start.as_usize(),
                )
                .unwrap();

            let len = elf_data.read_at(0, slice)?;

            boxed_elf = &mut slice[..len];

            ElfFile::new(boxed_elf)?
        };

        // No need to check the ELF magic number because it is already checked in `ElfFile::new`
        // let elf_magic = elf_header.pt1.magic;
        // '\x7fELF' in ASCII
        // const ELF_MAGIC: [u8; 4] = [0x7f, 0x45, 0x4c, 0x46];

        let mut min_start_vpn = VirtualPageNum::from_usize(usize::MAX);
        let mut max_end_vpn = VirtualPageNum::from_usize(0);

        let mut auxv = Vec::new();

        let mut implied_ph = VirtualAddress::null();
        let mut phdr = VirtualAddress::null();

        let mut interpreters = Vec::new();

        let mut pie_offset = 0;

        for ph in elf_info.program_iter() {
            debug!("Found program header: {ph:?}");

            match ph.get_type() {
                Ok(xmas_elf::program::Type::Load) => debug!("Loading"),
                Ok(xmas_elf::program::Type::Interp) => {
                    interpreters.push(ph);
                    debug!("Handle later");
                    continue;
                }
                Ok(xmas_elf::program::Type::Phdr) => {
                    phdr = VirtualAddress::from_usize(ph.virtual_addr() as usize);
                    debug!("Handled");
                    continue;
                }
                _ => {
                    warn!("skipping");
                    continue;
                }
            }

            let mut start = VirtualAddress::from_usize(ph.virtual_addr() as usize);
            let mut end = start + ph.mem_size() as usize;

            if start.to_floor_page_num().as_usize() == 0 {
                pie_offset = 4096;
            }

            if pie_offset != 0 {
                start += pie_offset;
                end += pie_offset;
            }

            if implied_ph.is_null() {
                implied_ph = start;
            }

            min_start_vpn = min_start_vpn.min(start.to_floor_page_num());
            max_end_vpn = max_end_vpn.max(end.to_floor_page_num());

            let mut segment_permissions = GenericMappingFlags::User | GenericMappingFlags::Kernel;

            if ph.flags().is_read() {
                segment_permissions |= GenericMappingFlags::Readable;
            }

            if ph.flags().is_write() || ph.get_type() == Ok(xmas_elf::program::Type::GnuRelro) {
                segment_permissions |= GenericMappingFlags::Writable;
            }

            if ph.flags().is_execute() {
                segment_permissions |= GenericMappingFlags::Executable;
            }

            let page_range = VirtualPageNumRange::from_start_end(
                start.to_floor_page_num(),
                end.to_ceil_page_num(), // end is exclusive
            );

            memory_space.alloc_and_map_area(MappingArea::new(
                page_range,
                AreaType::UserElf,
                MapType::Framed,
                segment_permissions,
                None,
            ));

            let data = &boxed_elf[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize];

            pt.lock().write_bytes(start, data).unwrap();
        }

        for interp in interpreters {
            log::warn!("interpreter found: {interp:?}")
            // TODO
        }

        // TODO: investigate this, certain section starts with the va of 0
        // e.g. testcase basic brk
        // debug_assert!(min_start_vpn > VirtualPageNum::from_usize(0));
        min_start_vpn = min_start_vpn.max(VirtualPageNum(1));

        attr.elf_area = VirtualAddressRange::from_start_end(
            min_start_vpn.start_addr(),
            max_end_vpn.start_addr(),
        );

        log::debug!("Elf segments loaded, max_end_vpn: {max_end_vpn:?}");

        if phdr.is_null() {
            phdr = implied_ph + elf_info.header.pt2.ph_offset() as usize
        }

        auxv.push(AuxVecEntry::new(AuxVecKey::AT_PHDR, phdr.as_usize()));
        auxv.push(AuxVecEntry::new(
            AuxVecKey::AT_PHENT,
            elf_info.header.pt2.ph_entry_size() as usize,
        ));
        auxv.push(AuxVecEntry::new(
            AuxVecKey::AT_PHNUM,
            elf_info.header.pt2.ph_count() as usize,
        ));
        auxv.push(AuxVecEntry::new(AuxVecKey::AT_PAGESZ, constants::PAGE_SIZE));
        auxv.push(AuxVecEntry::new(AuxVecKey::AT_BASE, 0));
        auxv.push(AuxVecEntry::new(AuxVecKey::AT_FLAGS, 0));
        auxv.push(AuxVecEntry::new(
            AuxVecKey::AT_ENTRY, // always the main program's entry point
            elf_info.header.pt2.entry_point() as usize,
        ));
        auxv.push(AuxVecEntry::new(AuxVecKey::AT_UID, 0));
        auxv.push(AuxVecEntry::new(AuxVecKey::AT_EUID, 0));
        auxv.push(AuxVecEntry::new(AuxVecKey::AT_GID, 0));
        auxv.push(AuxVecEntry::new(AuxVecKey::AT_EGID, 0));
        auxv.push(AuxVecEntry::new(AuxVecKey::AT_HWCAP, 0));
        // FIXME: Decouple the IMachine to separate crate and load the machine specific values
        auxv.push(AuxVecEntry::new(AuxVecKey::AT_CLKTCK, 125000000usize));
        auxv.push(AuxVecEntry::new(AuxVecKey::AT_SECURE, 0));

        // Reserved for signal trampoline
        max_end_vpn += 1;
        attr.signal_trampoline = max_end_vpn;

        max_end_vpn += 1;
        memory_space.alloc_and_map_area(MappingArea::new(
            VirtualPageNumRange::from_single(max_end_vpn),
            AreaType::UserStackGuardBase,
            MapType::Framed,
            GenericMappingFlags::empty(),
            None,
        ));
        attr.stack_guard_base =
            VirtualAddressRange::from_start_len(max_end_vpn.start_addr(), constants::PAGE_SIZE);

        let stack_page_count = constants::USER_STACK_SIZE / constants::PAGE_SIZE;
        max_end_vpn += 1;
        memory_space.alloc_and_map_area(MappingArea::new(
            VirtualPageNumRange::from_start_count(max_end_vpn, stack_page_count),
            AreaType::UserStack,
            MapType::Framed,
            GenericMappingFlags::User
                .union(GenericMappingFlags::Readable)
                .union(GenericMappingFlags::Writable),
            None,
        ));
        attr.stack_range = VirtualAddressRange::from_start_len(
            max_end_vpn.start_addr(),
            constants::USER_STACK_SIZE,
        );

        max_end_vpn += stack_page_count;
        let stack_top = max_end_vpn.start_addr();
        memory_space.alloc_and_map_area(MappingArea::new(
            VirtualPageNumRange::from_single(max_end_vpn),
            AreaType::UserStackGuardTop,
            MapType::Framed,
            GenericMappingFlags::empty(),
            None,
        ));
        attr.stack_gurad_top =
            VirtualAddressRange::from_start_len(max_end_vpn.start_addr(), constants::PAGE_SIZE);

        max_end_vpn += 1;
        memory_space.alloc_and_map_area(MappingArea::new(
            VirtualPageNumRange::from_start_count(max_end_vpn, 0),
            AreaType::UserBrk,
            MapType::Framed,
            GenericMappingFlags::User
                .union(GenericMappingFlags::Readable)
                .union(GenericMappingFlags::Writable),
            None,
        ));
        attr.brk_area_idx = memory_space
            .mappings()
            .iter()
            .enumerate()
            .find(|(_, area)| area.area_type == AreaType::UserBrk)
            .expect("UserBrk area not found")
            .0;
        attr.brk_start = max_end_vpn.start_addr();

        // FIXME: handle cases where there is a interpreter
        let entry_pc =
            VirtualAddress::from_usize(elf_info.header.pt2.entry_point() as usize) + pie_offset;

        #[cfg(debug_assertions)]
        {
            for area in memory_space.mappings() {
                let start = area.range.start().start_addr();
                let end = area.range.end().start_addr();

                let area_type = area.area_type;

                log::debug!("{area_type:?}: {start}..{end}");
            }

            let trampoline_page = attr.signal_trampoline;
            log::debug!(
                "SignalTrampoline: {}..{}",
                trampoline_page.start_addr(),
                trampoline_page.end_addr()
            );
        }

        unsafe {
            memory_space.init(attr);
        }

        Ok(LinuxLoader {
            memory_space,
            entry_pc,
            stack_top,
            argc: 0,
            argv_base: stack_top,
            envp_base: stack_top,
            auxv,
            executable: String::from(executable_path),
            command_line: Vec::new(),
        })
    }

    fn push<T: Copy>(&mut self, value: T) {
        // let kernel_pt = page_table::get_kernel_page_table();

        self.stack_top -= core::mem::size_of::<T>();
        self.stack_top = self.stack_top.align_down(core::mem::align_of::<T>());

        let pt = self.memory_space.mmu().lock();

        pt.export(self.stack_top, value).unwrap();
    }

    pub fn init_stack(&mut self, args: &[&str], envp: &[&str]) {
        let mut envps = Vec::new(); // envp pointers

        // Step1: Copy envp strings vector to the stack
        for env in envp.iter().rev() {
            self.push(0u8); // NULL-terminated
            for byte in env.bytes().rev() {
                self.push(byte);
            }
            envps.push(self.stack_top);
        }

        let mut argvs = Vec::new(); // argv pointers

        // Step2: Copy args strings vector to the stack
        for arg in args.iter().rev() {
            self.push(0u8); // NULL-terminated
            for byte in arg.bytes().rev() {
                self.push(byte);
            }
            argvs.push(self.stack_top);
        }

        // align stack top down to 8 bytes
        self.stack_top = self.stack_top.align_down(8);
        debug_assert!(self.stack_top.as_usize().is_multiple_of(8));

        // Step3: Copy PLATFORM string to the stack
        const PLATFORM: &str = "RISC-V64\0"; // FIXME
        const PLATFORM_LEN: usize = PLATFORM.len();

        // Ensure that start address of copied PLATFORM is aligned to 8 bytes
        self.stack_top -= PLATFORM_LEN;
        self.stack_top = self.stack_top.align_down(8);
        debug_assert!(self.stack_top.as_usize().is_multiple_of(8));
        self.stack_top += PLATFORM_LEN;

        for byte in PLATFORM.bytes().rev() {
            self.push(byte);
        }

        // Step4: Setup 16 random bytes for aux vector
        self.push(0xdeadbeefu64);
        self.push(0xdeadbeefu64);
        let aux_random_base = self.stack_top;

        // align down to 16 bytes
        self.stack_top = self.stack_top.align_down(16);
        debug_assert!(self.stack_top.as_usize().is_multiple_of(16));

        // Step5: setup aux vector
        self.push(AuxVecEntry::new(AuxVecKey::AT_NULL, 0));

        self.push(aux_random_base);
        self.push(AuxVecKey::AT_RANDOM);

        // Move auxv out of self
        let auxv = core::mem::take(&mut self.auxv);

        // Push other auxv entries
        for aux in auxv.iter().rev() {
            self.push(aux.value);
            self.push(aux.key);
        }

        // Step6: setup envp vector

        // push NULL for envp
        self.push(0usize);

        // push envp, envps is already in reverse order
        for env in envps.iter() {
            self.push(*env);
        }

        let envp_base = self.stack_top;

        // Step7: setup argv vector

        // push NULL for args
        self.push(0usize);

        // push args, argvs is already in reverse order
        for arg in argvs.iter() {
            self.push(*arg);
        }

        let argv_base = self.stack_top;

        // Step8: setup argc

        // push argc
        let argc = args.len();
        self.push(argc);

        // let argc_base = self.stack_top;

        self.argc = argc;
        self.argv_base = argv_base;
        self.envp_base = envp_base;

        self.command_line.push(self.executable.clone());
        for &arg in args {
            self.command_line.push(String::from(arg));
        }
    }
}
