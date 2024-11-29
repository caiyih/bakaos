use abstractions::IUsizeAlias;
use address::VirtualAddress;
use alloc::{slice, string::String, sync::Arc};
use filesystem::DummyFileSystem;
use filesystem_abstractions::{
    DirectoryEntryType, FileDescriptor, FileDescriptorBuilder, FileMode, FileStatistics,
    FrozenFileDescriptorBuilder, ICacheableFile, IInode, OpenFlags, PipeBuilder,
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
            .tcb
            .borrow_page_table()
            .guard_ptr(p_fd)
            .mustbe_user()
            .with_write()
        {
            Some(mut guard) => {
                let pipe_pair = PipeBuilder::open();

                let mut fd_table = ctx.tcb.fd_table.lock();

                match fd_table.allocate(pipe_pair.read_end_builder) {
                    Some(read_end) => guard.read_end = read_end as i32,
                    None => return Err(-1),
                }

                match fd_table.allocate(pipe_pair.write_end_builder) {
                    Some(write_end) => guard.write_end = write_end as i32,
                    None => {
                        fd_table.remove(guard.read_end as usize);
                        return Err(-1);
                    }
                }

                Ok(0)
            }
            None => Err(-1),
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
            return Err(-1);
        }

        match ctx
            .tcb
            .borrow_page_table()
            .guard_cstr(p_path, 1024)
            .must_have(PageTableEntryFlags::User | PageTableEntryFlags::Readable)
        {
            Some(guard) => {
                let dir_inode: Arc<dyn IInode> = if dirfd == FileDescriptor::AT_FDCWD {
                    let cwd = unsafe { ctx.tcb.cwd.get().as_ref().unwrap() };
                    filesystem_abstractions::lookup_inode(cwd).ok_or(-1isize)?
                } else {
                    let fd_table = ctx.tcb.fd_table.lock();
                    let fd = fd_table.get(dirfd as usize).ok_or(-1isize)?;
                    fd.access().inode().ok_or(-1isize)?
                };

                let path = core::str::from_utf8(&guard).map_err(|_| -1isize)?;
                let path = path::remove_relative_segments(path);
                let filename = path::get_filename(&path);
                let parent_inode_path = path::get_directory_name(&path).ok_or(-1isize)?;

                let inode: Arc<dyn IInode>;
                match dir_inode.lookup_recursive(&path) {
                    Ok(i) => inode = i,
                    Err(_) => {
                        if flags.contains(OpenFlags::O_CREAT) {
                            let parent_inode = dir_inode
                                .lookup_recursive(parent_inode_path)
                                .map_err(|_| -1isize)?;

                            let new_inode = parent_inode.touch(filename).map_err(|_| -1isize)?;

                            inode = new_inode;
                        } else {
                            return Err(-1);
                        }
                    }
                }

                let opened_file = filesystem_abstractions::open_file(inode, flags, 0).clear_type();

                let accessor = opened_file.cache_as_arc_accessor();

                let builder = FileDescriptorBuilder::new(accessor)
                    .set_readable()
                    .set_writable()
                    .freeze();

                let mut fd_table = ctx.tcb.fd_table.lock();
                match fd_table.allocate(builder) {
                    Some(fd) => Ok(fd as isize),
                    None => Err(-1),
                }
            }
            None => Err(-1),
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

        ctx.tcb.fd_table.lock().remove(fd); // rc to file will be dropped as the fd is removed
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

        let mut fd_table = ctx.tcb.fd_table.lock();
        match fd_table.get(fd) {
            Some(old) => {
                let builder = FrozenFileDescriptorBuilder::deconstruct(&old);
                match fd_table.allocate(builder) {
                    Some(newfd) => Ok(newfd as isize),
                    None => Err(-1),
                }
            }
            None => Err(-1),
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

        let mut fd_table = ctx.tcb.fd_table.lock();
        match fd_table.get(oldfd) {
            Some(old) => {
                let builder = FrozenFileDescriptorBuilder::deconstruct(&old);

                // if newfd is already open, close it
                if fd_table.get(newfd).is_some() {
                    fd_table.remove(newfd);
                }

                match fd_table.allocate_at(builder, newfd) {
                    Some(newfd) => Ok(newfd as isize),
                    None => Err(-1),
                }
            }
            None => Err(-1),
        }
    }

    fn name(&self) -> &str {
        "sys_dup3"
    }
}

pub struct MountSyscall;

impl ISyncSyscallHandler for MountSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let _source = ctx.arg0::<*const u8>();
        let target = ctx.arg1::<*const u8>();
        let _filesystemtype = ctx.arg2::<*const u8>();
        let _flags = ctx.arg3::<usize>();
        let _data = ctx.arg4::<*const u8>();

        match ctx
            .tcb
            .borrow_page_table()
            .guard_cstr(target, 1024)
            .must_have(PageTableEntryFlags::User | PageTableEntryFlags::Readable)
        {
            Some(guard) => {
                let mut target_path = core::str::from_utf8(&guard).map_err(|_| -1isize)?;

                let fully_qualified: String;
                if !path::is_path_fully_qualified(target_path) {
                    let cwd = unsafe { ctx.tcb.cwd.get().as_ref().unwrap() };
                    let full_path = path::get_full_path(target_path, Some(cwd)).ok_or(-1isize)?;
                    fully_qualified = path::remove_relative_segments(&full_path);
                    target_path = &fully_qualified;
                }

                filesystem_abstractions::mount_at(Arc::new(DummyFileSystem), target_path);

                Ok(0)
            }
            None => Err(-1),
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
            .tcb
            .borrow_page_table()
            .guard_cstr(target, 1024)
            .must_have(PageTableEntryFlags::User | PageTableEntryFlags::Readable)
        {
            Some(guard) => {
                let mut target_path = core::str::from_utf8(&guard).map_err(|_| -1isize)?;

                let fully_qualified: String;
                if !path::is_path_fully_qualified(target_path) {
                    let cwd = unsafe { ctx.tcb.cwd.get().as_ref().unwrap() };
                    let full_path = path::get_full_path(target_path, Some(cwd)).ok_or(-1isize)?;
                    fully_qualified = path::remove_relative_segments(&full_path);
                    target_path = &fully_qualified;
                }

                match filesystem_abstractions::umount_at(target_path) {
                    true => Ok(0),
                    false => Err(-1),
                }
            }
            None => Err(-1),
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
            return Err(-1);
        }

        match ctx
            .tcb
            .borrow_page_table()
            .guard_cstr(p_path, 1024)
            .must_have(PageTableEntryFlags::User | PageTableEntryFlags::Readable)
        {
            Some(guard) => {
                let dir_inode: Arc<dyn IInode> = if dirfd == FileDescriptor::AT_FDCWD {
                    let cwd = unsafe { ctx.tcb.cwd.get().as_ref().unwrap() };
                    filesystem_abstractions::lookup_inode(cwd).ok_or(-1isize)?
                } else {
                    let fd_table = ctx.tcb.fd_table.lock();
                    let fd = fd_table.get(dirfd as usize).ok_or(-1isize)?;
                    fd.access().inode().ok_or(-1isize)?
                };

                let path = core::str::from_utf8(&guard).map_err(|_| -1isize)?;
                let path = path::remove_relative_segments(path);
                let filename = path::get_filename(&path);
                let parent_inode_path = path::get_directory_name(&path).ok_or(-1isize)?;

                let parent_inode = dir_inode
                    .lookup_recursive(parent_inode_path)
                    .map_err(|_| -1isize)?;

                parent_inode.mkdir(filename).map_err(|_| -1isize)?;

                Ok(0)
            }
            None => Err(-1),
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
            return Err(-1);
        }

        let pt = ctx.tcb.borrow_page_table();

        match (
            pt.guard_cstr(p_path, 1024)
                .must_have(PageTableEntryFlags::User | PageTableEntryFlags::Readable),
            pt.guard_ptr(p_stat)
                .mustbe_user()
                .mustbe_readable()
                .with_write(),
        ) {
            (Some(path_guard), Some(mut buf_guard)) => {
                let dir_inode: Arc<dyn IInode> = if dirfd == FileDescriptor::AT_FDCWD {
                    let cwd = unsafe { ctx.tcb.cwd.get().as_ref().unwrap() };
                    filesystem_abstractions::lookup_inode(cwd).ok_or(-1isize)?
                } else {
                    let fd_table = ctx.tcb.fd_table.lock();
                    let fd = fd_table.get(dirfd as usize).ok_or(-1isize)?;
                    fd.access().inode().ok_or(-1isize)?
                };

                let path = core::str::from_utf8(&path_guard).map_err(|_| -1isize)?;
                let path = path::remove_relative_segments(path);

                let inode: Arc<dyn IInode> =
                    dir_inode.lookup_recursive(&path).map_err(|_| -1isize)?;

                inode.stat(&mut buf_guard).map_err(|_| -1isize).map(|_| 0)
            }
            _ => Err(-1),
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
            .tcb
            .borrow_page_table()
            .guard_ptr(p_buf)
            .mustbe_user()
            .mustbe_readable()
            .with_write()
        {
            Some(mut guard) => {
                let fd = ctx.tcb.fd_table.lock().get(fd).ok_or(-1isize)?;
                fd.access()
                    .inode()
                    .ok_or(-1isize)?
                    .stat(&mut guard)
                    .map_err(|_| -1isize)
                    .map(|_| 0)
            }
            None => Err(-1),
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

        let pt = ctx.tcb.borrow_page_table();

        match pt
            .guard_slice(buf)
            .mustbe_user()
            .mustbe_readable()
            .with_write()
        {
            Some(mut guard) => {
                let fd = ctx.tcb.fd_table.lock().get(fd).ok_or(-1isize)?;
                let inode = fd.access().inode().ok_or(-1isize)?;

                let entries = inode.read_dir().map_err(|_| -1isize)?;

                let mut offset = 0;

                for (idx, entry) in entries.iter().enumerate() {
                    let name = entry.filename.as_bytes();
                    let entry_size = core::mem::size_of::<LinuxDirEntry64>() + name.len() + 1;

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
                    p_entry.file_type = match entry.entry_type {
                        DirectoryEntryType::File => 0,
                        DirectoryEntryType::Directory => 1,
                    };

                    let name_slice =
                        unsafe { slice::from_raw_parts_mut(p_entry.name.as_mut_ptr(), name.len()) };
                    name_slice.copy_from_slice(name);

                    // Add null terminator
                    unsafe { p_entry.name.as_mut_ptr().add(name.len()).write(0) };

                    offset += entry_size;
                }

                Ok(fd.fd_idx() as isize)
            }
            None => Err(-1),
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
            return Err(-1);
        }

        match ctx
            .tcb
            .borrow_page_table()
            .guard_cstr(p_path, 1024)
            .must_have(PageTableEntryFlags::User | PageTableEntryFlags::Readable)
        {
            Some(guard) => {
                let dir_inode: Arc<dyn IInode> = if dirfd == FileDescriptor::AT_FDCWD {
                    let cwd = unsafe { ctx.tcb.cwd.get().as_ref().unwrap() };
                    filesystem_abstractions::lookup_inode(cwd).ok_or(-1isize)?
                } else {
                    let fd_table = ctx.tcb.fd_table.lock();
                    let fd = fd_table.get(dirfd as usize).ok_or(-1isize)?;
                    fd.access().inode().ok_or(-1isize)?
                };

                let path = core::str::from_utf8(&guard).map_err(|_| -1isize)?;
                let parent_path = path::get_directory_name(path).ok_or(-1isize)?;
                let filename = path::get_filename(path);

                let parent_inode = dir_inode
                    .lookup_recursive(parent_path)
                    .map_err(|_| -1isize)?;

                parent_inode
                    .remove(filename)
                    .map_err(|_| -1isize)
                    .map(|_| 0)
            }
            None => Err(-1),
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

        ctx.tcb
            .mmap(fd, flags, prot, offset, length)
            .ok_or(-1isize)
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

        match ctx.tcb.munmap(addr, length) {
            true => Ok(0),
            false => Err(-1),
        }
    }

    fn name(&self) -> &str {
        "sys_munmap"
    }
}
