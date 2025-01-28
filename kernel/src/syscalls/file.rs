use abstractions::IUsizeAlias;
use address::VirtualAddress;
use alloc::{slice, string::String, sync::Arc};
use constants::{ErrNo, SyscallError};
use filesystem_abstractions::{
    DirectoryTreeNode, FileDescriptor, FileDescriptorBuilder, FileMode, FileStatistics,
    FrozenFileDescriptorBuilder, ICacheableFile, OpenFlags, PipeBuilder,
};
use paging::{
    page_table::IOptionalPageGuardBuilderExtension, IWithPageGuardBuilder, MemoryMapFlags,
    MemoryMapProt, PageTableEntryFlags,
};

use super::{ISyncSyscallHandler, SyscallContext, SyscallResult};

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

                let mut fd_table = ctx.fd_table.lock();

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
            .must_have(PageTableEntryFlags::User | PageTableEntryFlags::Readable)
        {
            Some(guard) => {
                let dir_inode = if dirfd == FileDescriptor::AT_FDCWD {
                    let cwd = unsafe { ctx.cwd.get().as_ref().unwrap() };
                    filesystem_abstractions::global_open(cwd, None)
                        .map_err(|_| ErrNo::NoSuchFileOrDirectory)?
                } else {
                    let fd_table = ctx.fd_table.lock();
                    let fd = fd_table
                        .get(dirfd as usize)
                        .ok_or(ErrNo::BadFileDescriptor)?;
                    fd.access().inode().ok_or(ErrNo::FileDescriptorInBadState)?
                };

                let path = core::str::from_utf8(&guard).map_err(|_| ErrNo::InvalidArgument)?;
                let path = path::remove_relative_segments(path);
                let filename = path::get_filename(&path);

                let inode = if path::is_path_fully_qualified(&path) {
                    filesystem_abstractions::global_open(&path, None)
                        .map_err(|_| ErrNo::NoSuchFileOrDirectory)?
                } else {
                    let parent_inode_path =
                        path::get_directory_name(&path).ok_or(ErrNo::InvalidArgument)?;

                    match (
                        filesystem_abstractions::global_open(&path, Some(&dir_inode)),
                        flags.contains(OpenFlags::O_CREAT),
                    ) {
                        (Ok(i), _) => i,
                        (Err(_), true) => {
                            let parent_inode = filesystem_abstractions::global_open(
                                parent_inode_path,
                                Some(&dir_inode),
                            )
                            .map_err(|_| ErrNo::NoSuchFileOrDirectory)?;

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

                let mut fd_table = ctx.fd_table.lock();
                match fd_table.allocate(builder) {
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

        ctx.fd_table.lock().remove(fd); // rc to file will be dropped as the fd is removed
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

        let mut fd_table = ctx.fd_table.lock();
        match fd_table.get(fd) {
            Some(old) => {
                let builder = FrozenFileDescriptorBuilder::deconstruct(&old);
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

        let mut fd_table = ctx.fd_table.lock();
        match fd_table.get(oldfd) {
            Some(old) => {
                let builder = FrozenFileDescriptorBuilder::deconstruct(&old);

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
                .must_have(PageTableEntryFlags::User | PageTableEntryFlags::Readable),
            pt.guard_cstr(target, 1024)
                .must_have(PageTableEntryFlags::User | PageTableEntryFlags::Readable),
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
                    let cwd = unsafe { ctx.cwd.get().as_ref().unwrap() };
                    fully_qualified = path::get_full_path(target_path, Some(cwd))
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
            .must_have(PageTableEntryFlags::User | PageTableEntryFlags::Readable)
        {
            Some(guard) => {
                let mut target_path =
                    core::str::from_utf8(&guard).map_err(|_| ErrNo::InvalidArgument)?;

                let fully_qualified: String;
                if !path::is_path_fully_qualified(target_path) {
                    let cwd = unsafe { ctx.cwd.get().as_ref().unwrap() };
                    let full_path = path::get_full_path(target_path, Some(cwd))
                        .ok_or(ErrNo::InvalidArgument)?;
                    fully_qualified = path::remove_relative_segments(&full_path);
                    target_path = &fully_qualified;
                }

                match filesystem_abstractions::global_umount(target_path, None) {
                    Ok(_) => Ok(0),
                    Err(e) => e.to_syscall_error(),
                }
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
            .must_have(PageTableEntryFlags::User | PageTableEntryFlags::Readable)
        {
            Some(guard) => {
                let dir_inode = if dirfd == FileDescriptor::AT_FDCWD {
                    let cwd = unsafe { ctx.cwd.get().as_ref().unwrap() };
                    filesystem_abstractions::global_open(cwd, None)
                        .map_err(|_| ErrNo::NoSuchFileOrDirectory)?
                } else {
                    let fd_table = ctx.fd_table.lock();
                    let fd = fd_table
                        .get(dirfd as usize)
                        .ok_or(ErrNo::BadFileDescriptor)?;
                    fd.access().inode().ok_or(ErrNo::FileDescriptorInBadState)?
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
                .must_have(PageTableEntryFlags::User | PageTableEntryFlags::Readable),
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
                ) -> SyscallResult {
                    filesystem_abstractions::global_open(path, relative_to)
                        .map_err(|_| ErrNo::NoSuchFileOrDirectory)?
                        .stat(buf)
                        .map(|_| ErrNo::Success)
                        .map_err(|_| ErrNo::OperationNotPermitted)
                }

                let path = unsafe { core::str::from_utf8_unchecked(&path_guard) };

                if dirfd == FileDescriptor::AT_FDCWD {
                    let cwd = unsafe { ctx.cwd.get().as_ref().unwrap() };
                    let fullpath = path::combine(cwd, path).ok_or(ErrNo::InvalidArgument)?;
                    stat(&mut buf_guard, &fullpath, None)
                } else {
                    let fd_table = ctx.fd_table.lock();
                    let inode = fd_table
                        .get(dirfd as usize)
                        .and_then(|fd| fd.access().inode());
                    stat(&mut buf_guard, path, inode.as_ref())
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
                    .fd_table
                    .lock()
                    .get(fd)
                    .ok_or(ErrNo::BadFileDescriptor)?;
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

        let buf = unsafe { core::slice::from_raw_parts(p_buf, len) };

        let pt = ctx.borrow_page_table();

        match pt
            .guard_slice(buf)
            .mustbe_user()
            .mustbe_readable()
            .with_write()
        {
            Some(mut guard) => {
                let fd = ctx
                    .fd_table
                    .lock()
                    .get(fd)
                    .ok_or(ErrNo::BadFileDescriptor)?;
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
            .must_have(PageTableEntryFlags::User | PageTableEntryFlags::Readable)
        {
            Some(guard) => {
                let dir_inode = if dirfd == FileDescriptor::AT_FDCWD {
                    let cwd = unsafe { ctx.cwd.get().as_ref().unwrap() };
                    filesystem_abstractions::global_open(cwd, None)
                        .map_err(|_| ErrNo::NoSuchFileOrDirectory)?
                } else {
                    let fd_table = ctx.fd_table.lock();
                    let fd = fd_table
                        .get(dirfd as usize)
                        .ok_or(ErrNo::BadFileDescriptor)?;
                    fd.access().inode().ok_or(ErrNo::FileDescriptorInBadState)?
                };

                let path = core::str::from_utf8(&guard).map_err(|_| ErrNo::InvalidArgument)?;
                let parent_path = path::get_directory_name(path).ok_or(ErrNo::InvalidArgument)?;
                let filename = path::get_filename(path);

                let parent_inode =
                    filesystem_abstractions::global_open(parent_path, Some(&dir_inode))
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
        let addr = ctx.arg0::<*mut u8>();
        let length = ctx.arg1::<usize>();
        let prot = ctx.arg2::<MemoryMapProt>();
        let flags = ctx.arg3::<MemoryMapFlags>();
        let fd = ctx.arg4::<usize>();
        let offset = ctx.arg5::<usize>();

        debug_assert!(addr.is_null());

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

        ctx.fd_table
            .lock()
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
        let mut fd_table = ctx.fd_table.lock();

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
                    let builder = FrozenFileDescriptorBuilder::deconstruct(&fd);
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
