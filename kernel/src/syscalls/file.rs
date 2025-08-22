use core::cmp;

use abstractions::IUsizeAlias;
use address::VirtualAddress;
use alloc::{slice, string::String, sync::Arc};
use constants::{ErrNo, SyscallError};
use filesystem_abstractions::{
    global_open, global_open_raw, DirectoryTreeNode, FileDescriptor, FileDescriptorBuilder,
    FileMode, FileStatistics, FileStatisticsMode, FrozenFileDescriptorBuilder, ICacheableFile,
    IFile, OpenFlags, PipeBuilder,
};
use page_table::GenericMappingFlags;
use paging::{
    page_table::IOptionalPageGuardBuilderExtension, IWithPageGuardBuilder, MemoryMapFlags,
    MemoryMapProt,
};
use platform_specific::ISyscallContext;
use tasks::SyscallContext;
use timing::TimeSpec;

use super::{ISyncSyscallHandler, SyscallResult};

pub struct Pipe2Syscall;

impl ISyncSyscallHandler for Pipe2Syscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        #[repr(C)]
        struct FdPair {
            read_end: i32,
            write_end: i32,
        }

        let p_fd = ctx.arg0::<*mut FdPair>();

        match ctx
            .borrow_page_table()
            .guard_ptr(p_fd)
            .mustbe_user()
            .with_write()
        {
            Some(mut guard) => {
                let pipe_pair = PipeBuilder::open();

                let mut pcb = ctx.pcb.lock();
                let fd_table = &mut pcb.fd_table;

                match fd_table.allocate(pipe_pair.read_end_builder) {
                    Some(read_end) => guard.read_end = read_end as i32,
                    None => return SyscallError::TooManyOpenFiles,
                }

                match fd_table.allocate(pipe_pair.write_end_builder) {
                    Some(write_end) => guard.write_end = write_end as i32,
                    None => {
                        fd_table.remove(guard.read_end as usize);
                        return SyscallError::TooManyOpenFiles;
                    }
                }

                Ok(0)
            }
            None => SyscallError::BadAddress,
        }
    }

    fn name(&self) -> &str {
        "sys_pipe2"
    }
}

pub struct OpenAtSyscall;

impl ISyncSyscallHandler for OpenAtSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let dirfd = ctx.arg0::<isize>();
        let p_path = ctx.arg1::<*const u8>();
        let flags = ctx.arg2::<OpenFlags>();
        let _mode = ctx.arg3::<FileMode>();

        if dirfd < 0 && dirfd != FileDescriptor::AT_FDCWD {
            return Err(ErrNo::BadFileDescriptor);
        }

        match ctx
            .borrow_page_table()
            .guard_cstr(p_path, 1024)
            .must_have(GenericMappingFlags::User | GenericMappingFlags::Readable)
        {
            Some(guard) => {
                let dir_inode = {
                    let pcb = ctx.pcb.lock();

                    if dirfd == FileDescriptor::AT_FDCWD {
                        filesystem_abstractions::global_open(&pcb.cwd, None)
                            .map_err(|_| ErrNo::NoSuchFileOrDirectory)?
                    } else {
                        let fd = pcb
                            .fd_table
                            .get(dirfd as usize)
                            .ok_or(ErrNo::BadFileDescriptor)?;
                        fd.access().inode().ok_or(ErrNo::FileDescriptorInBadState)?
                    }
                };

                let path = core::str::from_utf8(&guard).map_err(|_| ErrNo::InvalidArgument)?;
                let path = path::remove_relative_segments(path);

                let inode = {
                    match (
                        filesystem_abstractions::global_open(&path, Some(&dir_inode)),
                        flags.contains(OpenFlags::O_CREAT),
                    ) {
                        (Ok(i), _) => i,
                        (Err(_), true) => {
                            let parent_inode_path =
                                path::get_directory_name(&path).ok_or(ErrNo::InvalidArgument)?;

                            let parent_inode = filesystem_abstractions::global_open(
                                parent_inode_path,
                                Some(&dir_inode),
                            )
                            .map_err(|_| ErrNo::NoSuchFileOrDirectory)?;

                            let filename = path::get_filename(&path);

                            parent_inode
                                .touch(filename)
                                .map_err(|_| ErrNo::OperationNotPermitted)?;

                            filesystem_abstractions::global_open(filename, Some(&parent_inode))
                                .map_err(|_| ErrNo::OperationCanceled)?
                        }
                        _ => return SyscallError::NoSuchFileOrDirectory,
                    }
                };

                let opened_file = inode.open_as_file(flags, 0).clear_type();

                let accessor = opened_file.cache_as_arc_accessor();

                let builder = FileDescriptorBuilder::new(accessor)
                    .set_readable()
                    .set_writable()
                    .freeze();

                match ctx.pcb.lock().fd_table.allocate(builder) {
                    Some(fd) => Ok(fd as isize),
                    None => SyscallError::BadFileDescriptor,
                }
            }
            None => SyscallError::BadAddress,
        }
    }

    fn name(&self) -> &str {
        "sys_openat"
    }
}

pub struct CloseSyscall;

impl ISyncSyscallHandler for CloseSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let fd = ctx.arg0::<usize>();

        ctx.pcb.lock().fd_table.remove(fd); // rc to file will be dropped as the fd is removed
                                            // and when rc is 0, the opened file will be dropped

        Ok(0)
    }

    fn name(&self) -> &str {
        "sys_close"
    }
}

pub struct DupSyscall;

impl ISyncSyscallHandler for DupSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let fd = ctx.arg0::<usize>();

        let mut pcb = ctx.pcb.lock();
        let fd_table = &mut pcb.fd_table;
        match fd_table.get(fd) {
            Some(old) => {
                let builder = FrozenFileDescriptorBuilder::deconstruct(old);
                match fd_table.allocate(builder) {
                    Some(newfd) => Ok(newfd as isize),
                    None => SyscallError::TooManyOpenFiles,
                }
            }
            None => SyscallError::BadFileDescriptor,
        }
    }

    fn name(&self) -> &str {
        "sys_dup"
    }
}

pub struct Dup3Syscall;

impl ISyncSyscallHandler for Dup3Syscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let oldfd = ctx.arg0::<usize>();
        let newfd = ctx.arg1::<usize>();
        let _flags = ctx.arg2::<usize>();

        if oldfd == newfd {
            return Ok(newfd as isize);
        }

        let mut pcb = ctx.pcb.lock();
        let fd_table = &mut pcb.fd_table;
        match fd_table.get(oldfd) {
            Some(old) => {
                let builder = FrozenFileDescriptorBuilder::deconstruct(old);

                // if newfd is already open, close it
                if fd_table.get(newfd).is_some() {
                    fd_table.remove(newfd);
                }

                match fd_table.allocate_at(builder, newfd) {
                    Some(newfd) => Ok(newfd as isize),
                    None => SyscallError::TooManyOpenFiles,
                }
            }
            None => SyscallError::BadFileDescriptor,
        }
    }

    fn name(&self) -> &str {
        "sys_dup3"
    }
}

pub struct MountSyscall;

impl ISyncSyscallHandler for MountSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let source = ctx.arg0::<*const u8>();
        let target = ctx.arg1::<*const u8>();
        let _filesystemtype = ctx.arg2::<*const u8>();
        let _flags = ctx.arg3::<usize>();
        let _data = ctx.arg4::<*const u8>();

        let pt = ctx.borrow_page_table();

        match (
            pt.guard_cstr(source, 1024)
                .must_have(GenericMappingFlags::User | GenericMappingFlags::Readable),
            pt.guard_cstr(target, 1024)
                .must_have(GenericMappingFlags::User | GenericMappingFlags::Readable),
        ) {
            (Some(source_guard), Some(target_guard)) => {
                let source_path =
                    core::str::from_utf8(&source_guard).map_err(|_| ErrNo::InvalidArgument)?;

                // TODO: get_fullpath with cwd
                if !path::is_path_fully_qualified(source_path) {
                    return SyscallError::InvalidArgument;
                }

                let mut target_path =
                    core::str::from_utf8(&target_guard).map_err(|_| ErrNo::InvalidArgument)?;

                let fully_qualified: String;
                if !path::is_path_fully_qualified(target_path) {
                    let pcb = ctx.pcb.lock();
                    fully_qualified = path::get_full_path(target_path, Some(&pcb.cwd))
                        .ok_or(ErrNo::InvalidArgument)?;
                    target_path = &fully_qualified;
                }

                let device = filesystem_abstractions::global_open(source_path, None)
                    .map_err(|_| ErrNo::NoSuchDevice)?;

                filesystem::global_mount_device_node(&device, target_path, None)
                    .map(|_| 0isize)
                    .map_err(|e| e.to_syscall_error().unwrap_err())
            }
            _ => SyscallError::BadAddress,
        }
    }

    fn name(&self) -> &str {
        "sys_mount"
    }
}

pub struct UmountSyscall;

impl ISyncSyscallHandler for UmountSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let target = ctx.arg0::<*const u8>();
        let _flags = ctx.arg1::<usize>();

        match ctx
            .borrow_page_table()
            .guard_cstr(target, 1024)
            .must_have(GenericMappingFlags::User | GenericMappingFlags::Readable)
        {
            Some(guard) => {
                let mut target_path =
                    core::str::from_utf8(&guard).map_err(|_| ErrNo::InvalidArgument)?;

                let fully_qualified: String;
                if !path::is_path_fully_qualified(target_path) {
                    let pcb = ctx.pcb.lock();
                    let full_path = path::get_full_path(target_path, Some(&pcb.cwd))
                        .ok_or(ErrNo::InvalidArgument)?;
                    fully_qualified = path::remove_relative_segments(&full_path);
                    target_path = &fully_qualified;
                }

                let _ = filesystem_abstractions::global_umount(target_path, None);

                Ok(0)
            }
            None => SyscallError::BadAddress,
        }
    }

    fn name(&self) -> &str {
        "sys_umount"
    }
}

pub struct MkdirAtSyscall;

impl ISyncSyscallHandler for MkdirAtSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let dirfd = ctx.arg0::<isize>();
        let p_path = ctx.arg1::<*const u8>();
        let _mode = ctx.arg2::<FileMode>();

        if dirfd < 0 && dirfd != FileDescriptor::AT_FDCWD {
            return SyscallError::BadFileDescriptor;
        }

        match ctx
            .borrow_page_table()
            .guard_cstr(p_path, 1024)
            .must_have(GenericMappingFlags::User | GenericMappingFlags::Readable)
        {
            Some(guard) => {
                let dir_inode = {
                    let pcb = ctx.pcb.lock();

                    if dirfd == FileDescriptor::AT_FDCWD {
                        filesystem_abstractions::global_open(&pcb.cwd, None)
                            .map_err(|_| ErrNo::NoSuchFileOrDirectory)?
                    } else {
                        let fd = pcb
                            .fd_table
                            .get(dirfd as usize)
                            .ok_or(ErrNo::BadFileDescriptor)?;
                        fd.access().inode().ok_or(ErrNo::FileDescriptorInBadState)?
                    }
                };

                let path = core::str::from_utf8(&guard).map_err(|_| ErrNo::InvalidArgument)?;
                let path = path::remove_relative_segments(path);
                let filename = path::get_filename(&path);
                let parent_inode_path =
                    path::get_directory_name(&path).ok_or(ErrNo::InvalidArgument)?;

                let parent_inode =
                    filesystem_abstractions::global_open(parent_inode_path, Some(&dir_inode))
                        .map_err(|_| ErrNo::NoSuchFileOrDirectory)?;

                parent_inode
                    .mkdir(filename)
                    .map_err(|_| ErrNo::OperationNotPermitted)?;

                Ok(0)
            }
            None => SyscallError::BadAddress,
        }
    }

    fn name(&self) -> &str {
        "sys_mkdirat"
    }
}

pub struct NewFstatatSyscall;

impl ISyncSyscallHandler for NewFstatatSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let dirfd = ctx.arg0::<isize>();
        let p_path = ctx.arg1::<*const u8>();
        let p_stat = ctx.arg2::<*mut FileStatistics>();

        if dirfd < 0 && dirfd != FileDescriptor::AT_FDCWD {
            return SyscallError::BadFileDescriptor;
        }

        let pt = ctx.borrow_page_table();

        match (
            pt.guard_cstr(p_path, 1024)
                .must_have(GenericMappingFlags::User | GenericMappingFlags::Readable),
            pt.guard_ptr(p_stat)
                .mustbe_user()
                .mustbe_readable()
                .with_write(),
        ) {
            (Some(path_guard), Some(mut buf_guard)) => {
                fn stat(
                    buf: &mut FileStatistics,
                    path: &str,
                    relative_to: Option<&Arc<DirectoryTreeNode>>,
                    resolve_link: bool,
                ) -> SyscallResult {
                    match resolve_link {
                        true => filesystem_abstractions::global_open(path, relative_to),
                        false => filesystem_abstractions::global_open_raw(path, relative_to),
                    }
                    .map_err(|_| ErrNo::NoSuchFileOrDirectory)?
                    .stat(buf)
                    .map(|_| ErrNo::Success)
                    .map_err(|_| ErrNo::OperationNotPermitted)
                }

                const AT_SYMLINK_NOFOLLOW: i32 = 0x100;
                let flag = ctx.arg3::<i32>();

                let resolve_link = (flag & AT_SYMLINK_NOFOLLOW) == 0;

                let path = unsafe { core::str::from_utf8_unchecked(&path_guard) };

                let pcb = ctx.pcb.lock();
                if dirfd == FileDescriptor::AT_FDCWD {
                    let fullpath = path::combine(&pcb.cwd, path);
                    stat(&mut buf_guard, &fullpath, None, resolve_link)
                } else {
                    let inode = pcb
                        .fd_table
                        .get(dirfd as usize)
                        .and_then(|fd| fd.access().inode());
                    stat(&mut buf_guard, path, inode.as_ref(), resolve_link)
                }
            }
            _ => SyscallError::BadAddress,
        }
    }

    fn name(&self) -> &str {
        "sys_newfstatat"
    }
}

pub struct NewFstatSyscall;

impl ISyncSyscallHandler for NewFstatSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let fd = ctx.arg0::<usize>();
        let p_buf = ctx.arg1::<*mut FileStatistics>();

        match ctx
            .borrow_page_table()
            .guard_ptr(p_buf)
            .mustbe_user()
            .mustbe_readable()
            .with_write()
        {
            Some(mut guard) => {
                let fd = ctx
                    .pcb
                    .lock()
                    .fd_table
                    .get(fd)
                    .ok_or(ErrNo::BadFileDescriptor)?
                    .clone();
                fd.access()
                    .inode()
                    .ok_or(ErrNo::FileDescriptorInBadState)?
                    .stat(&mut guard)
                    .map_err(|_| ErrNo::OperationNotPermitted)
                    .map(|_| 0)
            }
            None => SyscallError::BadAddress,
        }
    }

    fn name(&self) -> &str {
        "sys_newfstat"
    }
}

pub struct GetDents64Syscall;

impl ISyncSyscallHandler for GetDents64Syscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        #[repr(C)]
        struct LinuxDirEntry64 {
            inode_id: u64,
            doffsset: u64,
            entry_len: u16,
            file_type: u8,
            name: [u8; 0],
        }

        let fd = ctx.arg0::<usize>();
        let p_buf = ctx.arg1::<*mut u8>();
        let len = ctx.arg2::<usize>();

        let pt = ctx.borrow_page_table();

        match pt
            .guard_slice(p_buf, len)
            .mustbe_user()
            .mustbe_readable()
            .with_write()
        {
            Some(mut guard) => {
                let fd = ctx
                    .pcb
                    .lock()
                    .fd_table
                    .get(fd)
                    .ok_or(ErrNo::BadFileDescriptor)?
                    .clone();
                let file = fd.access();
                let file_meta = file.metadata().ok_or(ErrNo::FileDescriptorInBadState)?;

                let entries = file_meta.read_dir().ok_or(ErrNo::NotADirectory)?;

                unsafe { slice::from_raw_parts_mut(p_buf, len).fill(0) };

                let ptr = p_buf as usize;
                let mut offset: usize = 0;

                let mut idx = file_meta.offset();
                if idx < entries.len() {
                    for entry in entries[idx..].iter() {
                        let name = entry.filename.as_bytes();
                        let mut entry_size: usize =
                            core::mem::size_of::<LinuxDirEntry64>() + name.len() + 1;
                        entry_size = ((entry_size + ptr) | 7) + 1 - ptr; // align to 8 bytes

                        if offset + entry_size > len {
                            break;
                        }

                        let p_entry = unsafe {
                            &mut *guard
                                .as_mut()
                                .as_mut_ptr()
                                .add(offset)
                                .cast::<LinuxDirEntry64>()
                        };

                        p_entry.inode_id = idx as u64;
                        p_entry.doffsset = offset as u64; // no meaning for user space
                        p_entry.entry_len = entry_size as u16;
                        p_entry.file_type = entry.entry_type as u8;

                        let name_slice = unsafe {
                            slice::from_raw_parts_mut(p_entry.name.as_mut_ptr(), name.len())
                        };
                        name_slice.copy_from_slice(name);

                        // Add null terminator. Not needed, as the whole buffer is zeroed
                        // unsafe { p_entry.name.as_mut_ptr().add(name.len()).write_volatile(0) };

                        idx += 1;

                        offset += entry_size;
                        file_meta.set_offset(idx);
                    }
                }

                Ok(offset as isize)
            }
            None => SyscallError::BadAddress,
        }
    }

    fn name(&self) -> &str {
        "sys_getdents64"
    }
}

pub struct UnlinkAtSyscall;

impl ISyncSyscallHandler for UnlinkAtSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let dirfd = ctx.arg0::<isize>();
        let p_path = ctx.arg1::<*const u8>();
        let _flags = ctx.arg2::<usize>();

        if dirfd < 0 && dirfd != FileDescriptor::AT_FDCWD {
            return SyscallError::BadFileDescriptor;
        }

        match ctx
            .borrow_page_table()
            .guard_cstr(p_path, 1024)
            .must_have(GenericMappingFlags::User | GenericMappingFlags::Readable)
        {
            Some(guard) => {
                let dir_inode = {
                    let pcb = ctx.pcb.lock();

                    if dirfd == FileDescriptor::AT_FDCWD {
                        filesystem_abstractions::global_open(&pcb.cwd, None).ok()
                    } else {
                        let fd = pcb
                            .fd_table
                            .get(dirfd as usize)
                            .ok_or(ErrNo::BadFileDescriptor)?;
                        Some(fd.access().inode().ok_or(ErrNo::FileDescriptorInBadState)?)
                    }
                };

                let path = core::str::from_utf8(&guard).map_err(|_| ErrNo::InvalidArgument)?;
                let parent_path = path::get_directory_name(path).ok_or(ErrNo::InvalidArgument)?;
                let filename = path::get_filename(path);

                let parent_inode =
                    filesystem_abstractions::global_open(parent_path, dir_inode.as_ref())
                        .map_err(|_| ErrNo::NoSuchFileOrDirectory)?;

                parent_inode
                    .remove(filename)
                    .map_err(|_| ErrNo::NoSuchFileOrDirectory)
                    .map(|_| 0)
            }
            None => SyscallError::BadAddress,
        }
    }

    fn name(&self) -> &str {
        "sys_unlinkat"
    }
}

pub struct MmapSyscall;

impl ISyncSyscallHandler for MmapSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let _addr = ctx.arg0::<*mut u8>();
        let length = ctx.arg1::<usize>();
        let prot = ctx.arg2::<MemoryMapProt>();
        let flags = ctx.arg3::<MemoryMapFlags>();
        let fd = ctx.arg4::<usize>();
        let offset = ctx.arg5::<usize>();

        ctx.mmap(fd, flags, prot, offset, length)
            .ok_or(ErrNo::OperationNotPermitted) // TODO: check this
            .map(|addr| addr.as_usize() as isize)
    }

    fn name(&self) -> &str {
        "sys_old_mmap"
    }
}

pub struct MunmapSyscall;

impl ISyncSyscallHandler for MunmapSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let addr = ctx.arg0::<VirtualAddress>();
        let length = ctx.arg1::<usize>();

        match ctx.munmap(addr, length) {
            true => Ok(0),
            false => SyscallError::InvalidArgument,
        }
    }

    fn name(&self) -> &str {
        "sys_munmap"
    }
}

pub struct IoControlSyscall;

impl ISyncSyscallHandler for IoControlSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        const TIOCGPGRP: i32 = 0x540f;
        const TIOCGWINSZ: i32 = 0x5413;

        let fd = ctx.arg0::<usize>();
        let op = ctx.arg1::<i32>();
        let argp = ctx.arg2::<*mut u8>();

        ctx.pcb
            .lock()
            .fd_table
            .get(fd)
            .ok_or(ErrNo::BadFileDescriptor)
            .map(|_| 0)?;

        match op {
            TIOCGPGRP | TIOCGWINSZ => unsafe { *argp = 0 },
            _ => (),
        }

        Ok(0)
    }

    fn name(&self) -> &str {
        "sys_ioctl"
    }
}

pub struct FileControlSyscall;

impl ISyncSyscallHandler for FileControlSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        const F_DUPFD: usize = 0;
        const F_GETFD: usize = 1;
        const F_SETFD: usize = 2;
        const F_GETFL: usize = 3;
        const F_SETFL: usize = 4;
        const F_DUPFD_CLOEXEC: usize = 1030;

        let fd_idx = ctx.arg0::<usize>();
        let mut pcb = ctx.pcb.lock();
        let fd_table = &mut pcb.fd_table;

        let arg = ctx.arg2::<usize>();
        match ctx.arg1::<usize>() /* arg */ {
            F_SETFL => match fd_table.get(fd_idx) {
                Some(fd) => {
                    let flags = OpenFlags::from_bits_truncate(arg);
                    match fd.access().set_flags(flags) {
                        true => Ok(0),
                        false => SyscallError::FileDescriptorInBadState,
                    }
                }
                None => SyscallError::BadFileDescriptor,
            },
            F_GETFD | F_GETFL => match fd_table.get(fd_idx) {
                Some(fd) => Ok(fd.access().flags().bits() as isize),
                None => SyscallError::BadFileDescriptor,
            },
            F_DUPFD | F_DUPFD_CLOEXEC => match fd_table.get(fd_idx) {
                Some(fd) => {
                    let builder = FrozenFileDescriptorBuilder::deconstruct(fd);
                    match fd_table.allocate(builder) {
                        Some(id) => Ok(id as isize),
                        None => SyscallError::TooManyOpenFiles,
                    }
                }
                None => SyscallError::BadFileDescriptor,
            },
            F_SETFD => Ok(0),
            op => {
                log::warn!("fnctl: Unsupported operation: {op}");
                SyscallError::InvalidArgument
            }
        }
    }

    fn name(&self) -> &str {
        "sys_fnctl"
    }
}

pub struct ReadLinkAtSyscall;

impl ISyncSyscallHandler for ReadLinkAtSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let dirfd = ctx.arg0::<isize>();
        let p_path = ctx.arg1::<*const u8>();
        let p_buf = ctx.arg2::<*mut u8>();
        let len = ctx.arg3::<usize>();

        if dirfd < 0 && dirfd != FileDescriptor::AT_FDCWD {
            return SyscallError::BadFileDescriptor;
        }

        let pt = ctx.borrow_page_table();

        match (
            pt.guard_cstr(p_path, 1024)
                .must_have(GenericMappingFlags::User)
                .with_read(),
            pt.guard_slice(p_buf, len)
                .mustbe_user()
                .mustbe_readable()
                .with_write(),
        ) {
            (Some(path), Some(mut buf)) => {
                let path = unsafe { core::str::from_utf8_unchecked(&path) };
                let dir = resolve_dirfd_path(ctx, dirfd, path)?;

                let node = global_open_raw(path, dir.as_ref())
                    .map_err(|_| ErrNo::NoSuchFileOrDirectory)?;

                let target = node.resolve_link().ok_or(ErrNo::InvalidArgument)?;
                let bytes = target.as_bytes();

                let len = cmp::min(len, bytes.len());

                buf.as_mut()[..len].copy_from_slice(&bytes[..len]);

                Ok(len as isize)
            }
            _ => SyscallError::BadAddress,
        }
    }

    fn name(&self) -> &str {
        "sys_readlinkat"
    }
}

pub struct SymbolLinkAtSyscall;

impl ISyncSyscallHandler for SymbolLinkAtSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let p_existing = ctx.arg0::<*const u8>();
        let dirfd = ctx.arg1::<isize>();
        let p_linkto = ctx.arg2::<*const u8>();

        let pt = ctx.borrow_page_table();

        match (
            pt.guard_cstr(p_existing, 1024)
                .must_have(GenericMappingFlags::User)
                .with_read(),
            pt.guard_cstr(p_linkto, 1024)
                .must_have(GenericMappingFlags::User)
                .with_read(),
        ) {
            (Some(existing), Some(linkto)) => {
                let existing = unsafe { core::str::from_utf8_unchecked(&existing) };
                let linkto = unsafe { core::str::from_utf8_unchecked(&linkto) };

                let dir = resolve_dirfd_path(ctx, dirfd, linkto)?;

                let parent_path = path::get_directory_name(linkto).unwrap_or_default();
                let name = path::get_filename(linkto);

                let parent_node =
                    global_open(parent_path, dir.as_ref()).map_err(|e| e.to_errno())?;

                parent_node
                    .soft_link(name, existing)
                    .map_err(|e| e.to_errno())
                    .map(|_| 0)
            }
            _ => SyscallError::BadAddress,
        }
    }

    fn name(&self) -> &str {
        "sys_symlinkat"
    }
}

fn resolve_dirfd_path(
    ctx: &SyscallContext,
    dirfd: isize,
    path: &str,
) -> Result<Option<Arc<DirectoryTreeNode>>, isize> {
    match path::is_path_fully_qualified(path) {
        true => Ok(None),
        false if dirfd >= 0 => Ok(Some(
            ctx.pcb
                .lock()
                .fd_table
                .get(dirfd as usize)
                .and_then(|fd| fd.access().inode())
                .ok_or(ErrNo::BadFileDescriptor)?,
        )),
        false if dirfd == FileDescriptor::AT_FDCWD => {
            let cwd = &ctx.pcb.lock().cwd;
            global_open(cwd, None).map_err(|e| e.to_errno()).map(Some)
        }
        _ => Err(ErrNo::BadFileDescriptor),
    }
}

pub struct LinkAtSyscall;

impl ISyncSyscallHandler for LinkAtSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        const AT_SYMLINK_FOLLOW: i32 = 0x400;
        const AT_EMPTY_PATH: i32 = 0x1000;

        fn check_and_get_paths(
            ctx: &SyscallContext,
            oldpath_ptr: *const u8,
            newpath_ptr: *const u8,
        ) -> Result<(&'static str, &'static str), isize> {
            let pt = ctx.borrow_page_table();

            let oldpath = pt
                .guard_cstr(oldpath_ptr, 1024)
                .must_have(GenericMappingFlags::User)
                .with_read()
                .ok_or(ErrNo::BadAddress)?;

            let newpath = pt
                .guard_cstr(newpath_ptr, 1024)
                .must_have(GenericMappingFlags::User)
                .with_read()
                .ok_or(ErrNo::BadAddress)?;

            unsafe {
                // make reference 'static
                let oldpath = core::slice::from_raw_parts(oldpath.as_ptr(), oldpath.len());
                let newpath = core::slice::from_raw_parts(newpath.as_ptr(), newpath.len());

                Ok((
                    core::str::from_utf8_unchecked(oldpath),
                    core::str::from_utf8_unchecked(newpath),
                ))
            }
        }

        fn create_hard_link(
            parent_path: &str,
            base: Option<&Arc<DirectoryTreeNode>>,
            name: &str,
            inode: &Arc<DirectoryTreeNode>,
        ) -> SyscallResult {
            let new_parent = global_open(parent_path, base).map_err(|e| e.to_errno())?;
            new_parent
                .hard_link(name, inode)
                .map_err(|e| e.to_errno())?;
            Ok(0)
        }

        let olddirfd = ctx.arg0::<isize>();
        let oldpath_ptr = ctx.arg1::<*const u8>();
        let newdirfd = ctx.arg2::<isize>();
        let newpath_ptr = ctx.arg3::<*const u8>();
        let flags = ctx.arg4::<i32>();

        let (oldpath, newpath) = check_and_get_paths(ctx, oldpath_ptr, newpath_ptr)?;

        if (flags & AT_EMPTY_PATH) != 0 {
            if !oldpath.is_empty() {
                return SyscallError::InvalidArgument;
            }

            let pcb = ctx.pcb.lock();
            let old_fd = pcb
                .fd_table
                .get(olddirfd as usize)
                .ok_or(ErrNo::BadFileDescriptor)?;
            let inode = old_fd.access().inode().ok_or(ErrNo::BadFileDescriptor)?;

            let parent_path = path::get_directory_name(newpath).unwrap_or_default();
            let name = path::get_filename(newpath);

            let new_parent_base = resolve_dirfd_path(ctx, newdirfd, newpath)?;
            return create_hard_link(parent_path, new_parent_base.as_ref(), name, &inode);
        }

        let follow = (flags & AT_SYMLINK_FOLLOW) != 0;

        let old_base = resolve_dirfd_path(ctx, olddirfd, oldpath)?;
        let mut old_inode =
            global_open_raw(oldpath, old_base.as_ref()).map_err(|e| e.to_errno())?;

        if follow {
            old_inode = old_inode.resolve_all_link().map_err(|e| e.to_errno())?;
        }

        let parent_path = path::get_directory_name(newpath).unwrap_or_default();
        let name = path::get_filename(newpath);

        let new_parent_base = resolve_dirfd_path(ctx, newdirfd, newpath)?;
        create_hard_link(parent_path, new_parent_base.as_ref(), name, &old_inode)
    }

    fn name(&self) -> &str {
        "sys_linkat"
    }
}

pub struct LongSeekSyscall;

impl ISyncSyscallHandler for LongSeekSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let pcb = ctx.pcb.lock();

        let fd = ctx.arg0::<usize>();
        let fd = pcb.fd_table.get(fd).ok_or(ErrNo::BadFileDescriptor)?;

        let file_metadata = fd.access().metadata().ok_or(ErrNo::IllegalSeek)?;

        let offset = ctx.arg1::<i64>();
        let whence = ctx.arg2::<usize>();

        match file_metadata.seek(offset, whence) {
            true => Ok(file_metadata.offset() as isize),
            _ => SyscallError::IllegalSeek,
        }
    }

    fn name(&self) -> &str {
        "sys_lseek"
    }
}

pub struct FileTruncateSyscall;

impl ISyncSyscallHandler for FileTruncateSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let pcb = ctx.pcb.lock();

        let fd = ctx.arg0::<usize>();
        let fd = pcb.fd_table.get(fd).ok_or(ErrNo::BadFileDescriptor)?;

        let file_metadata = fd
            .access()
            .metadata()
            .ok_or(ErrNo::FileDescriptorInBadState)?;

        let new_size = ctx.arg1::<u64>();

        file_metadata
            .inode()
            .resize_inode(new_size)
            .map_err(|_| ErrNo::FileDescriptorInBadState)
            .map(|s| s as isize)
    }

    fn name(&self) -> &str {
        "sys_ftruncate64"
    }
}

pub struct SocketSyscall;

impl ISyncSyscallHandler for SocketSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        struct DummyFile;

        impl IFile for DummyFile {
            fn can_read(&self) -> bool {
                true
            }

            fn can_write(&self) -> bool {
                true
            }
        }

        let socket: Arc<dyn IFile> = Arc::new(DummyFile);
        let accessor = socket.cache_as_arc_accessor();

        let mut pcb = ctx.pcb.lock();

        match pcb.fd_table.allocate(
            FileDescriptorBuilder::new(accessor)
                .set_readable()
                .set_writable(),
        ) {
            Some(fd) => Ok(fd as isize),
            None => SyscallError::TooManyOpenFiles,
        }
    }

    fn name(&self) -> &str {
        "sys_socket"
    }
}

pub struct StatxSyscall;

impl ISyncSyscallHandler for StatxSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        struct StatxTimestamp {
            _tv_sec: i64,
            _tv_nsec: u32,
            __reserved: i32,
        }

        impl From<TimeSpec> for StatxTimestamp {
            fn from(value: TimeSpec) -> Self {
                Self {
                    _tv_sec: value.tv_sec,
                    _tv_nsec: value.tv_nsec as u32,
                    __reserved: 0,
                }
            }
        }

        #[repr(C)]
        struct Statx {
            /* 0x00 */
            stx_mask: u32,       /* What results were written [uncond] */
            stx_blksize: u32,    /* Preferred general I/O size [uncond] */
            stx_attributes: u64, /* Flags conveying information about the file [uncond] */
            /* 0x10 */
            stx_nlink: u32, /* Number of hard links */
            stx_uid: u32,   /* User ID of owner */
            stx_gid: u32,   /* Group ID of owner */
            stx_mode: u16,  /* File mode */
            __spare0: u16,
            /* 0x20 */
            stx_ino: u64,             /* Inode number */
            stx_size: u64,            /* File size */
            stx_blocks: u64,          /* Number of 512-byte blocks allocated */
            stx_attributes_mask: u64, /* Mask to show what's supported in stx_attributes */
            /* 0x40 */
            stx_atime: StatxTimestamp, /* Last access time */
            stx_btime: StatxTimestamp, /* File creation time */
            stx_ctime: StatxTimestamp, /* Last attribute change time */
            stx_mtime: StatxTimestamp, /* Last data modification time */
            /* 0x80 */
            _stx_rdev_major: u32, /* Device ID of special file [if bdev/cdev] */
            _stx_rdev_minor: u32,
            _stx_dev_major: u32, /* ID of device containing file [uncond] */
            _stx_dev_minor: u32,
            /* 0x90 */
            _stx_mnt_id: u64,
            _stx_dio_mem_align: u32, /* Memory buffer alignment for direct I/O */
            _stx_dio_offset_align: u32, /* File offset alignment for direct I/O */
            /* 0xa0 */
            __spare3: [u64; 12], /* Spare space for future expansion */
                                 /* 0x100 */
        }

        let dirfd = ctx.arg0::<isize>();
        let path = ctx.arg1::<*const u8>();

        if dirfd < 0 && dirfd != FileDescriptor::AT_FDCWD {
            return SyscallError::BadFileDescriptor;
        }

        let pt = ctx.borrow_page_table();

        let p_stat = ctx.arg4::<*mut Statx>();

        match (
            pt.guard_cstr(path, 1024)
                .must_have(GenericMappingFlags::User | GenericMappingFlags::Readable),
            pt.guard_ptr(p_stat)
                .mustbe_user()
                .mustbe_readable()
                .with_write(),
        ) {
            (Some(path_guard), Some(mut buf_guard)) => {
                fn stat(
                    buf: &mut Statx,
                    path: &str,
                    relative_to: Option<&Arc<DirectoryTreeNode>>,
                    resolve_link: bool,
                ) -> SyscallResult {
                    let mut stat = unsafe { core::mem::zeroed() };

                    let fnode = match resolve_link {
                        true => filesystem_abstractions::global_open(path, relative_to),
                        false => filesystem_abstractions::global_open_raw(path, relative_to),
                    }
                    .map_err(|_| ErrNo::NoSuchFileOrDirectory)?;

                    fnode
                        .stat(&mut stat)
                        .map_err(|_| ErrNo::OperationNotPermitted)?;

                    buf.stx_attributes_mask = 0;
                    buf.stx_mask = 0x7ff;
                    buf.stx_blksize = stat.block_size;

                    buf.stx_uid = 0;
                    buf.stx_gid = 0;

                    buf.stx_nlink = Ord::max(stat.link_count, 1);
                    buf.stx_mode =
                        unsafe { core::mem::transmute::<FileStatisticsMode, u32>(stat.mode) }
                            as u16;

                    buf.stx_ino = stat.inode_id;
                    buf.stx_size = stat.size;
                    buf.stx_blocks = stat.block_count;

                    buf.stx_atime = stat.atime.into();
                    buf.stx_btime = stat.ctime.into();
                    buf.stx_ctime = stat.mtime.into();
                    buf.stx_mtime = stat.mtime.into();

                    buf.stx_attributes = 0;

                    let fs = fnode.get_containing_filesystem();

                    if Arc::ptr_eq(&fs, &fnode) {
                        const STATX_ATTR_MOUNT_ROOT: u64 = 0x2000;
                        buf.stx_attributes |= STATX_ATTR_MOUNT_ROOT;
                    }

                    Ok(0)
                }

                const AT_SYMLINK_NOFOLLOW: i32 = 0x100;
                let flag = ctx.arg2::<i32>();

                let resolve_link = (flag & AT_SYMLINK_NOFOLLOW) == 0;

                let path = unsafe { core::str::from_utf8_unchecked(&path_guard) };

                let pcb = ctx.pcb.lock();
                if dirfd == FileDescriptor::AT_FDCWD {
                    let fullpath = path::combine(&pcb.cwd, path);
                    stat(&mut buf_guard, &fullpath, None, resolve_link)
                } else {
                    let inode = pcb
                        .fd_table
                        .get(dirfd as usize)
                        .and_then(|fd| fd.access().inode());
                    stat(&mut buf_guard, path, inode.as_ref(), resolve_link)
                }
            }
            _ => SyscallError::BadAddress,
        }
    }

    fn name(&self) -> &str {
        "sys_statx"
    }
}

fn extract_parent_path<'a>(
    path: &'a str,
    dirfd: &Option<Arc<DirectoryTreeNode>>,
    cwd: &'a str,
) -> &'a str {
    match (path::get_directory_name(path), dirfd) {
        (Some(parent), _) if !parent.is_empty() => parent,
        (_, Some(_)) => path::CURRENT_DIRECTORY,
        _ => cwd,
    }
}

pub struct RenameAt2Syscall;

impl ISyncSyscallHandler for RenameAt2Syscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        const PATH_MAX_LEN: usize = 256;
        const PAGE_FLAGS: GenericMappingFlags =
            GenericMappingFlags::User.union(GenericMappingFlags::Readable);

        let old_dirfd = ctx.arg0::<usize>();
        let old_path = ctx.arg1::<*const u8>();
        let new_dirfd = ctx.arg2::<usize>();
        let new_path = ctx.arg3::<*const u8>();

        macro_rules! get_fd_from_inode {
            ($pcb:ident, $fd:ident) => {
                match $fd as isize {
                    FileDescriptor::AT_FDCWD => None,
                    _ => $pcb
                        .fd_table
                        .get($fd)
                        .ok_or(ErrNo::BadFileDescriptor)?
                        .access_ref()
                        .inode(),
                }
            };
        }

        let pcb = ctx.pcb.lock();

        let old_dirfd = get_fd_from_inode!(pcb, old_dirfd);
        let new_dirfd = get_fd_from_inode!(pcb, new_dirfd);

        match (
            ctx.borrow_page_table()
                .guard_cstr(old_path, PATH_MAX_LEN)
                .must_have(PAGE_FLAGS),
            ctx.borrow_page_table()
                .guard_cstr(new_path, PATH_MAX_LEN)
                .must_have(PAGE_FLAGS),
        ) {
            (Some(old_path), Some(new_path)) => {
                let old_path = unsafe { core::str::from_utf8_unchecked(&old_path) };
                let new_path = unsafe { core::str::from_utf8_unchecked(&new_path) };

                let (old_parent, old_name) = (
                    extract_parent_path(old_path, &old_dirfd, &pcb.cwd),
                    path::get_filename(old_path),
                );

                let old_parent = global_open(old_parent, old_dirfd.as_ref())
                    .map_err(|_| ErrNo::NoSuchFileOrDirectory)?;

                let (new_parent, new_name) = (
                    extract_parent_path(new_path, &new_dirfd, &pcb.cwd),
                    path::get_filename(new_path),
                );

                let new_parent = global_open(new_parent, new_dirfd.as_ref())
                    .map_err(|_| ErrNo::NoSuchFileOrDirectory)?;

                if Arc::ptr_eq(&old_parent, &new_parent) {
                    return old_parent
                        .rename(old_name, new_name)
                        .map(|_| 0)
                        .map_err(|e| e.to_errno());
                } else {
                    // FIXME: we have to move the node to the new parent
                    #[cfg(debug_assertions)]
                    unimplemented!();

                    #[cfg(not(debug_assertions))]
                    return Ok(0);
                }

                #[allow(unreachable_code)]
                SyscallError::Success
            }
            _ => SyscallError::BadAddress,
        }
    }

    fn name(&self) -> &str {
        "sys_renameat2"
    }
}
