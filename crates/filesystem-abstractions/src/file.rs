use core::cell::UnsafeCell;
use core::ops::Deref;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    DirectoryEntry, DirectoryEntryType, DirectoryTreeNode, FileCacheAccessor, OpenFlags,
    TeleTypewriterBuilder,
};
use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::vec::Vec;
use downcast_rs::{impl_downcast, DowncastSync};
use hermit_sync::{RawSpinMutex, SpinMutex};
use lock_api::MutexGuard;

pub struct FileMetadata {
    open_offset: AtomicUsize,
    open_flags: SpinMutex<OpenFlags>,
    inode: Arc<DirectoryTreeNode>,
    children_entries: UnsafeCell<Option<Vec<DirectoryEntry>>>,
}

impl FileMetadata {
    pub fn open(inode: Arc<DirectoryTreeNode>, flags: OpenFlags, offset: usize) -> Self {
        Self {
            open_offset: AtomicUsize::new(offset),
            open_flags: SpinMutex::new(flags),
            inode,
            children_entries: UnsafeCell::new(None),
        }
    }

    pub fn offset(&self) -> usize {
        self.open_offset.load(Ordering::Relaxed)
    }

    pub fn set_offset(&self, offset: usize) {
        self.open_offset.store(offset, Ordering::Relaxed);
    }

    pub fn flags(&self) -> MutexGuard<RawSpinMutex, OpenFlags> {
        self.open_flags.lock()
    }

    pub fn set_flags(&self, new_flags: OpenFlags) {
        *self.open_flags.lock() = new_flags
    }

    pub fn inode(&self) -> Arc<DirectoryTreeNode> {
        self.inode.clone()
    }

    pub fn read_dir(&self) -> Option<&[DirectoryEntry]> {
        let children_entries = unsafe { self.children_entries.get().as_mut().unwrap() };
        if let Some(ref children) = children_entries {
            return Some(children);
        }

        if let Ok(entries) = self.inode.read_dir() {
            *children_entries = Some(entries);

            return children_entries.as_deref();
        }

        None
    }
}

unsafe impl Send for FileMetadata {}
unsafe impl Sync for FileMetadata {}

pub trait IFile: DowncastSync + Send + Sync {
    fn metadata(&self) -> Option<Arc<FileMetadata>>;

    fn can_read(&self) -> bool {
        self.metadata().is_some()
    }

    fn can_write(&self) -> bool {
        self.metadata()
            .is_some_and(|metadata| metadata.flags().contains(OpenFlags::O_WRONLY))
    }

    fn read_avaliable(&self) -> bool {
        true
    }

    fn write_avaliable(&self) -> bool {
        true
    }

    fn flags(&self) -> OpenFlags {
        *self.metadata().unwrap().flags()
    }

    fn set_flags(&self, new_flags: OpenFlags) -> bool {
        match self.metadata() {
            Some(metadata) => {
                metadata.set_flags(new_flags);
                true
            }
            None => false,
        }
    }

    fn inode(&self) -> Option<Arc<DirectoryTreeNode>> {
        self.metadata().map(|metadata| metadata.inode())
    }

    fn is_dir(&self) -> bool {
        self.inode().unwrap().metadata().entry_type == DirectoryEntryType::Directory
    }

    fn lseek(&self, offset: usize) -> usize {
        self.metadata().map_or(0, |metadata| {
            metadata.set_offset(offset);
            offset
        })
    }

    fn write(&self, buf: &[u8]) -> usize {
        self.metadata().map_or(0, |metadata| {
            metadata
                .inode()
                .writeat(metadata.offset(), buf)
                .map_or(0, |written| written)
        })
    }

    fn read(&self, buf: &mut [u8]) -> usize {
        self.metadata().map_or(0, |metadata| {
            metadata
                .inode()
                .readat(metadata.offset(), buf)
                .map_or(0, |read| read)
        })
    }
}

impl_downcast!(sync IFile);

#[derive(Debug, Clone)]
pub struct FrozenFileDescriptor {
    file_handle: Arc<FileCacheAccessor>, // file handle
    can_read: bool,                      // whether the file descriptor is readable
    can_write: bool,                     // whether the file descriptor is writable
}

/// `FileDescriptor` represents an open file in a task's file descriptor table.
/// It holds metadata about the file, including its file handle and access permissions.
/// It also supports redirection to another file descriptor.
#[derive(Debug)]
pub struct FileDescriptor {
    idx: usize,                  // index in the task's file descriptor table
    inner: FrozenFileDescriptor, // file handle and access permissions, used to trace dupped fds
}

unsafe impl Send for FileDescriptor {}
unsafe impl Sync for FileDescriptor {}

impl FileDescriptor {
    pub const AT_FDCWD: isize = -100;

    /// Returns a weak reference to the current file descriptor.
    pub fn weak_clone(self: &Arc<FileDescriptor>) -> Weak<FileDescriptor> {
        Arc::downgrade(self)
    }

    /// Returns the index of the file descriptor in the task's file descriptor table.
    pub fn fd_idx(self: &Arc<FileDescriptor>) -> usize {
        self.idx
    }

    /// Returns the file handle associated with the file descriptor, following any redirections.
    pub fn file_handle<'a>(self: &'a Arc<FileDescriptor>) -> &'a Arc<FileCacheAccessor> {
        &self.inner.file_handle
    }

    /// Checks if the file descriptor is readable, following any redirections.
    pub fn can_read(self: &Arc<FileDescriptor>) -> bool {
        self.inner.can_read
    }

    /// Checks if the file descriptor is writable, following any redirections.
    pub fn can_write(self: &Arc<FileDescriptor>) -> bool {
        self.inner.can_write
    }
}

impl Clone for FileDescriptor {
    // This clone the file descriptor and sharing the same FrozenFileDescriptor.
    // No permission changes are allowed after the file descriptor is cloned as they are shared.
    // To create a new file descriptor with different permissions, use `FileDescriptorBuilder`.
    // But this loses the ability to trace the original file descriptor as they are not shared the same FrozenFileDescriptor.
    fn clone(&self) -> Self {
        Self {
            idx: self.idx,
            inner: self.inner.clone(),
        }
    }
}

impl Deref for FileDescriptor {
    type Target = FileCacheAccessor;

    fn deref(&self) -> &Self::Target {
        &self.inner.file_handle
    }
}

#[allow(private_bounds)] // Hide abstractions from the public interface.
pub trait IFileDescriptorBuilder: IHasFrozenFileDescriptor {
    /// Builds the `FileDescriptor` with the specified index.
    /// # Arguments
    /// * `idx` - The index of the file descriptor in the task's file descriptor table.
    fn build(&self, idx: usize) -> Arc<FileDescriptor> {
        Arc::new(FileDescriptor {
            idx,
            inner: self.inner(),
        })
    }

    /// Builds the `FileDescriptor` with an independent file handle.
    /// This will create a new file handle for the file descriptor, that means the file descriptor is not shared with the original one.
    fn build_non_inherited(&self, idx: usize) -> Arc<FileDescriptor> {
        let file_handle = self.inner().file_handle.clone_non_inherited_arc();
        Arc::new(FileDescriptor {
            idx,
            inner: FrozenFileDescriptor {
                file_handle,
                can_read: self.inner().can_read,
                can_write: self.inner().can_write,
            },
        })
    }
}

trait IHasFrozenFileDescriptor {
    fn inner(&self) -> FrozenFileDescriptor;
}

/// Builder for creating `FileDescriptor` instances with customizable properties
/// This deconstructs an existing `FileDescriptor` and allows for changing its properties.
/// But you will also lose the ability to trace the original file descriptor as they are not shared the same FrozenFileDescriptor.
/// To trace the original file descriptor, use `FrozenFileDescriptorBuilder`.
pub struct FileDescriptorBuilder {
    fd_inner: FrozenFileDescriptor,
}

unsafe impl Send for FileDescriptorBuilder {}
unsafe impl Sync for FileDescriptorBuilder {}

impl FileDescriptorBuilder {
    /// Creates a new `FileDescriptorBuilder` with the given file handle.
    /// # Arguments
    /// * `file_handle` - The file handle associated with the file descriptor.
    pub fn new(file_handle: Arc<FileCacheAccessor>) -> Self {
        FileDescriptorBuilder {
            fd_inner: FrozenFileDescriptor {
                file_handle,
                can_read: false,
                can_write: false,
            },
        }
    }

    pub fn deconstruct(fd: &Arc<FileDescriptor>) -> Self {
        FileDescriptorBuilder {
            fd_inner: FrozenFileDescriptor {
                file_handle: fd.inner.file_handle.clone(),
                can_read: fd.can_read(),
                can_write: fd.can_write(),
            },
        }
    }

    /// Sets the file descriptor to be readable.
    pub fn set_readable(mut self) -> Self {
        self.fd_inner.can_read = true;
        self
    }

    /// Sets the file descriptor to be writable.
    pub fn set_writable(mut self) -> Self {
        self.fd_inner.can_write = true;
        self
    }

    // Freezes the builder and returns a `FrozenPermissionFileDescriptorBuilder`.
    // which prohibits further permission changes but still allows building the file descriptor.
    pub fn freeze(self) -> FrozenFileDescriptorBuilder {
        FrozenFileDescriptorBuilder {
            fd_inner: self.fd_inner,
        }
    }
}

impl IFileDescriptorBuilder for FileDescriptorBuilder {}

impl IHasFrozenFileDescriptor for FileDescriptorBuilder {
    fn inner(&self) -> FrozenFileDescriptor {
        self.fd_inner.clone()
    }
}

#[derive(Clone)]
pub struct FrozenFileDescriptorBuilder {
    fd_inner: FrozenFileDescriptor,
}

impl FrozenFileDescriptorBuilder {
    pub fn new(fd_inner: FrozenFileDescriptor) -> Self {
        Self { fd_inner }
    }

    pub fn deconstruct(fd: &Arc<FileDescriptor>) -> Self {
        Self::new(fd.inner.clone())
    }

    pub fn fd_inner(&self) -> &FrozenFileDescriptor {
        &self.fd_inner
    }

    pub fn unfreeze(self) -> FileDescriptorBuilder {
        FileDescriptorBuilder {
            fd_inner: self.fd_inner,
        }
    }
}

impl IFileDescriptorBuilder for FrozenFileDescriptorBuilder {}

impl IHasFrozenFileDescriptor for FrozenFileDescriptorBuilder {
    fn inner(&self) -> FrozenFileDescriptor {
        self.fd_inner.clone()
    }
}

#[derive(Debug)]
pub struct FileDescriptorTable {
    table: Vec<Option<Arc<FileDescriptor>>>,
}

impl FileDescriptorTable {
    pub fn clone_for(&self, task_id: usize) -> Self {
        let mut new = Self::new(task_id);

        for entry in self.table.iter().skip(3) {
            new.table.push(entry.clone());
        }

        debug_assert!(self.table.len() == new.table.len());

        new
    }
}

impl FileDescriptorTable {
    /// Creates a new `FileDescriptorTable` with the given capacity.
    /// # Arguments
    /// * `task_id` - The ID of the task that owns the file descriptor table.
    pub fn new(task_id: usize) -> Self {
        let tty = TeleTypewriterBuilder::open_for(task_id);

        let mut table = FileDescriptorTable { table: Vec::new() };

        table.allocate(tty.stdin_builder);
        table.allocate(tty.stdout_builder);
        table.allocate(tty.stderr_builder);

        table
    }

    /// Returns the file descriptor at the specified index.
    /// # Arguments
    /// * `idx` - The index of the file descriptor in the table.
    pub fn get(&self, idx: usize) -> Option<Arc<FileDescriptor>> {
        self.table.get(idx).cloned().flatten()
    }

    /// Sets the file descriptor at the specified index.
    /// # Arguments
    /// * `idx` - The index of the file descriptor in the table.
    /// * `fd` - The file descriptor to set.
    pub fn set(&mut self, idx: usize, fd: Arc<FileDescriptor>) {
        self.table[idx] = Some(fd);
    }

    /// Removes the file descriptor at the specified index.
    /// # Arguments
    /// * `idx` - The index of the file descriptor in the table.
    pub fn remove(&mut self, idx: usize) {
        if idx < self.table.len() {
            self.table[idx] = None;
        }
    }

    /// Allocates a new file descriptor in the table with the given properties.
    /// # Arguments
    /// * `fd_builder` - The builder for creating the file descriptor.
    pub fn allocate<TFDBuilder: IFileDescriptorBuilder>(
        &mut self,
        fd_builder: TFDBuilder,
    ) -> Option<usize> {
        for (idx, entry) in self.table.iter().enumerate() {
            if entry.is_none() {
                self.table[idx] = Some(fd_builder.build(idx));
                return Some(idx);
            }
        }

        if self.table.len() >= Self::MAX_SIZE {
            return None;
        }

        self.table.push(Some(fd_builder.build(self.table.len())));
        Some(self.table.len() - 1)
    }

    pub const MAX_SIZE: usize = 42; // according to rlimit
    pub fn allocate_at<TFDBuilder: IFileDescriptorBuilder>(
        &mut self,
        fd_builder: TFDBuilder,
        idx: usize,
    ) -> Option<usize> {
        if idx >= Self::MAX_SIZE {
            return None;
        }

        if self.table.len() <= idx {
            self.table.reserve_exact(idx - self.table.len() + 1);
            self.table.resize_with(idx + 1, || None);
        } else if self.table.get(idx)?.is_some() {
            return None;
        }

        self.table[idx] = Some(fd_builder.build(idx));
        Some(idx)
    }
}

pub struct OpenedDiskInode {
    pub(crate) metadata: Arc<FileMetadata>,
}

impl IFile for OpenedDiskInode {
    fn metadata(&self) -> Option<Arc<FileMetadata>> {
        Some(self.metadata.clone())
    }
}

impl OpenedDiskInode {
    pub fn clear_type(self: Arc<Self>) -> Arc<dyn IFile> {
        self
    }
}
