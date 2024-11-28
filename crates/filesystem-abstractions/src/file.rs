use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{DirectoryEntryType, IInode, OpenFlags};
use crate::{IStdioFile, Stderr, Stdin, Stdout};
use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::{vec, vec::Vec};
use hermit_sync::{RawSpinMutex, RwSpinLock, SpinMutex};
use lock_api::{MappedMutexGuard, MutexGuard};

pub struct FileMetadata {
    open_offset: AtomicUsize,
    open_flags: SpinMutex<OpenFlags>,
    inode: Arc<dyn IInode>,
}

impl FileMetadata {
    pub fn open(inode: Arc<dyn IInode>, flags: OpenFlags, offset: usize) -> Self {
        Self {
            open_offset: AtomicUsize::new(offset),
            open_flags: SpinMutex::new(flags),
            inode,
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

    pub fn inode(&self) -> Arc<dyn IInode> {
        self.inode.clone()
    }
}

unsafe impl Send for FileMetadata {}
unsafe impl Sync for FileMetadata {}

pub trait IFile: Send + Sync {
    fn metadata(&self) -> Option<Arc<FileMetadata>>;

    fn can_read(&self) -> bool {
        self.metadata().is_some()
    }

    fn can_write(&self) -> bool {
        self.metadata().map_or(false, |metadata| {
            metadata.flags().contains(OpenFlags::O_WRONLY)
        })
    }

    fn avaliable(&self) -> bool {
        true
    }

    fn flags(&self) -> OpenFlags {
        self.metadata().unwrap().flags().clone()
    }

    fn delete(&self) -> bool {
        false
    }

    fn inode(&self) -> Option<Arc<dyn IInode>> {
        self.metadata().map(|metadata| metadata.inode())
    }

    fn is_dir(&self) -> bool {
        self.inode().unwrap().metadata().map_or(false, |metadata| {
            metadata.entry_type == DirectoryEntryType::Directory
        })
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

static mut FILE_TABLE: SpinMutex<Vec<Option<Arc<dyn IFile>>>> = SpinMutex::new(Vec::new());

pub trait ICacheableFile: IFile {
    fn cache_as_arc_accessor(self: &Arc<Self>) -> Arc<FileCacheAccessor>;
    fn cache_as_accessor(self: &Arc<Self>) -> FileCacheAccessor;
}

impl ICacheableFile for dyn IFile {
    fn cache_as_arc_accessor(self: &Arc<Self>) -> Arc<FileCacheAccessor> {
        Arc::new(self.cache_as_accessor())
    }

    fn cache_as_accessor(self: &Arc<Self>) -> FileCacheAccessor {
        FileCacheAccessor::cache(self.clone())
    }
}

#[derive(Debug)]
pub struct FileCacheAccessor {
    file_id: usize,
}

impl FileCacheAccessor {
    pub fn cache(file: Arc<dyn IFile>) -> FileCacheAccessor {
        let mut caches = unsafe { FILE_TABLE.lock() };

        let file_id = match caches.iter().enumerate().find(|x| x.1.is_none()) {
            Some((index, _)) => {
                caches[index] = Some(file);
                index
            }
            None => {
                caches.push(Some(file));
                caches.len() - 1
            }
        };

        FileCacheAccessor { file_id }
    }

    pub fn access(&self) -> Arc<dyn IFile> {
        let caches = unsafe { FILE_TABLE.lock() };
        caches[self.file_id].as_ref().unwrap().clone()
    }

    pub fn file_id(&self) -> usize {
        self.file_id
    }

    /// Returns a mutable reference to the file in the cache.
    /// # Safety
    /// Made this function unsafe because it allows mutable access to the file in the cache table.
    /// The caller must ensure that the mutable reference is not used concurrently.
    ///
    /// # Returns
    /// A mutable reference to the file in the cache table.
    pub unsafe fn access_mut(&self) -> MappedMutexGuard<'static, RawSpinMutex, Arc<dyn IFile>> {
        let caches = FILE_TABLE.lock();
        MutexGuard::map(caches, |caches| caches[self.file_id].as_mut().unwrap())
    }
}

/// `FileDescriptor` represents an open file in a task's file descriptor table.
/// It holds metadata about the file, including its file handle and access permissions.
/// It also supports redirection to another file descriptor.
#[derive(Debug)]
pub struct FileDescriptor {
    idx: usize,                          // index in the task's file descriptor table
    file_handle: Arc<FileCacheAccessor>, // file handle
    can_read: bool,                      // whether the file descriptor is readable
    can_write: bool,                     // whether the file descriptor is writable
    redirected_fd: RwSpinLock<Option<Weak<FileDescriptor>>>, // redirected file descriptor
}

unsafe impl Send for FileDescriptor {}
unsafe impl Sync for FileDescriptor {}

impl FileDescriptor {
    /// Returns the real file descriptor that this file descriptor points to, following any redirections.
    pub fn real_fd(self: &Arc<FileDescriptor>) -> Arc<FileDescriptor> {
        let mut current = self.clone();
        loop {
            let weak = {
                let read_guard = current.redirected_fd.read();
                if let Some(weak) = read_guard.as_ref().map(|w| w.clone()) {
                    weak
                } else {
                    break;
                }
            };
            // If the weak pointer is still valid, we can upgrade it to a strong pointer.
            // But if not, we should stop redirection and return the current file descriptor.
            match weak.upgrade() {
                Some(fd) => current = fd,
                None => *current.redirected_fd.write() = None,
            }
        }
        current
    }

    /// Returns a weak reference to the current file descriptor.
    pub fn weak_clone(self: &Arc<FileDescriptor>) -> Weak<FileDescriptor> {
        Arc::downgrade(self)
    }

    /// Checks if the file descriptor is redirected to another file descriptor.
    pub fn is_redirected(self: &Arc<FileDescriptor>) -> bool {
        self.redirected_fd.read().is_some()
    }

    /// Redirects the current file descriptor to another file descriptor.
    /// # Arguments
    /// * `new_fd` - The file descriptor to redirect to.
    pub fn redirect_to(self: &Arc<FileDescriptor>, new_fd: Arc<FileDescriptor>) {
        *self.redirected_fd.write() = Some(new_fd.weak_clone());
    }

    /// Returns the index of the file descriptor in the task's file descriptor table.
    pub fn id(self: &Arc<FileDescriptor>) -> usize {
        self.idx
    }

    /// Returns the index of the real file descriptor, following any redirections.
    pub fn real_id(self: &Arc<FileDescriptor>) -> usize {
        self.real_fd().id()
    }

    /// Returns the file handle associated with the file descriptor, following any redirections.
    pub fn file_handle(self: &Arc<FileDescriptor>) -> Arc<FileCacheAccessor> {
        self.real_fd().file_handle.clone()
    }

    /// Checks if the file descriptor is readable, following any redirections.
    pub fn can_read(self: &Arc<FileDescriptor>) -> bool {
        self.real_fd().can_read
    }

    /// Checks if the file descriptor is writable, following any redirections.
    pub fn can_write(self: &Arc<FileDescriptor>) -> bool {
        self.real_fd().can_write
    }
}

pub trait IFileDescriptorBuilder {
    /// Builds the `FileDescriptor` with the specified index.
    /// # Arguments
    /// * `idx` - The index of the file descriptor in the task's file descriptor table.
    fn build(self, idx: usize) -> Arc<FileDescriptor>;
}

/// Builder for creating `FileDescriptor` instances with customizable properties.
pub struct FileDescriptorBuilder {
    file_handle: Arc<FileCacheAccessor>,
    can_read: bool,
    can_write: bool,
}

unsafe impl Send for FileDescriptorBuilder {}
unsafe impl Sync for FileDescriptorBuilder {}

impl FileDescriptorBuilder {
    /// Creates a new `FileDescriptorBuilder` with the given file handle.
    /// # Arguments
    /// * `file_handle` - The file handle associated with the file descriptor.
    pub fn new(file_handle: Arc<FileCacheAccessor>) -> Self {
        FileDescriptorBuilder {
            file_handle,
            can_read: false,
            can_write: false,
        }
    }

    pub fn from_existing(fd: &Arc<FileDescriptor>) -> Self {
        FileDescriptorBuilder {
            file_handle: fd.file_handle.clone(),
            can_read: fd.can_read,
            can_write: fd.can_write,
        }
    }

    /// Sets the file descriptor to be readable.
    pub fn set_readable(mut self) -> Self {
        self.can_read = true;
        self
    }

    /// Sets the file descriptor to be writable.
    pub fn set_writable(mut self) -> Self {
        self.can_write = true;
        self
    }

    // Freezes the builder and returns a `FrozenPermissionFileDescriptorBuilder`.
    // which prohibits further permission changes but still allows building the file descriptor.
    pub fn freeze(self) -> FrozenPermissionFileDescriptorBuilder {
        FrozenPermissionFileDescriptorBuilder::new(self)
    }
}

impl IFileDescriptorBuilder for FileDescriptorBuilder {
    /// Builds the `FileDescriptor` with the specified index.
    /// # Arguments
    /// * `idx` - The index of the file descriptor in the task's file descriptor table.
    fn build(self, idx: usize) -> Arc<FileDescriptor> {
        Arc::new(FileDescriptor {
            idx,
            file_handle: self.file_handle,
            can_read: self.can_read,
            can_write: self.can_write,
            redirected_fd: RwSpinLock::new(None),
        })
    }
}

pub struct FrozenPermissionFileDescriptorBuilder {
    builder: FileDescriptorBuilder,
}

impl FrozenPermissionFileDescriptorBuilder {
    pub fn new(builder: FileDescriptorBuilder) -> Self {
        FrozenPermissionFileDescriptorBuilder { builder }
    }
}

impl IFileDescriptorBuilder for FrozenPermissionFileDescriptorBuilder {
    fn build(self, idx: usize) -> Arc<FileDescriptor> {
        self.builder.build(idx)
    }
}

#[derive(Debug)]
pub struct FileDescriptorTable {
    table: Vec<Option<Arc<FileDescriptor>>>,
}

impl FileDescriptorTable {
    pub fn clone_for(&self, task_id: usize) -> Self {
        let mut new = Self::new(task_id);

        for entry in self.table.iter() {
            new.table.push(entry.clone());
        }

        new
    }
}

impl FileDescriptorTable {
    /// Creates a new `FileDescriptorTable` with the given capacity.
    /// # Arguments
    /// * `task_id` - The ID of the task that owns the file descriptor table.
    pub fn new(task_id: usize) -> Self {
        FileDescriptorTable {
            table: vec![
                Some(
                    FileDescriptorBuilder::new(Stdin::open_for(task_id).cache_as_arc_accessor())
                        .set_readable()
                        .build(0),
                ),
                Some(
                    FileDescriptorBuilder::new(Stdout::open_for(task_id).cache_as_arc_accessor())
                        .set_writable()
                        .build(1),
                ),
                Some(
                    FileDescriptorBuilder::new(Stderr::open_for(task_id).cache_as_arc_accessor())
                        .set_writable()
                        .build(2),
                ),
            ],
        }
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
        self.table[idx] = None;
    }

    /// Allocates a new file descriptor in the table with the given properties.
    /// # Arguments
    /// * `fd_builder` - The builder for creating the file descriptor.
    pub fn allocate(&mut self, fd_buider: FileDescriptorBuilder) -> usize {
        for (idx, entry) in self.table.iter().enumerate() {
            if entry.is_none() {
                self.table[idx] = Some(fd_buider.build(idx));
                return idx;
            }
        }

        self.table.push(Some(fd_buider.build(self.table.len())));
        self.table.len() - 1
    }
}
