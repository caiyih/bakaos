use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{DirectoryEntry, DirectoryEntryType, DirectoryTreeNode, OpenFlags};
use alloc::sync::Arc;
use alloc::vec::Vec;
use downcast_rs::{impl_downcast, Downcast, DowncastSend};
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

    pub fn seek(&self, offset: i64, whence: usize) -> bool {
        const WHENCE_START: usize = 0;
        const WHENCE_CURRENT: usize = 1;
        const WHENCE_END: usize = 2;

        match whence {
            WHENCE_START => self.set_offset(offset as usize),
            WHENCE_CURRENT => {
                if offset >= 0 {
                    self.open_offset
                        .fetch_add(offset as usize, Ordering::Relaxed);
                } else {
                    self.open_offset
                        .fetch_sub((-offset) as usize, Ordering::Relaxed);
                }
            }
            WHENCE_END => {
                let inode_metadata = self.inode.metadata();

                if inode_metadata.entry_type == DirectoryEntryType::File
                    || inode_metadata.entry_type == DirectoryEntryType::BlockDevice
                {
                    let end = self.inode.metadata().size;
                    let offset = end as i64 + offset;
                    self.set_offset(offset as usize);
                } else {
                    return false;
                }
            }
            _ => return false,
        }

        true
    }

    pub fn offset(&self) -> usize {
        self.open_offset.load(Ordering::Relaxed)
    }

    pub fn set_offset(&self, offset: usize) {
        self.open_offset.store(offset, Ordering::Relaxed);
    }

    pub fn flags(&self) -> MutexGuard<'_, RawSpinMutex, OpenFlags> {
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

pub trait IFile: Downcast + DowncastSend + Send + Sync {
    fn metadata(&self) -> Option<Arc<FileMetadata>> {
        None
    }

    fn can_read(&self) -> bool {
        self.metadata().is_some()
    }

    fn can_write(&self) -> bool {
        self.metadata().is_some_and(|metadata| {
            let flag = metadata.flags();
            flag.contains(OpenFlags::O_WRONLY) || flag.contains(OpenFlags::O_RDWR)
        })
    }

    fn read_avaliable(&self) -> bool {
        true
    }

    fn write_avaliable(&self) -> bool {
        true
    }

    fn flags(&self) -> OpenFlags {
        self.metadata().map_or(OpenFlags::NONE, |m| *m.flags())
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

    fn write(&self, buf: &[u8]) -> usize {
        self.metadata().map_or(0, |metadata| {
            metadata
                .inode()
                .writeat(metadata.offset(), buf)
                .map_or(0, |written| {
                    metadata.seek(written as i64, 1 /* SEEK_CURRENT */);
                    written
                })
        })
    }

    fn read(&self, buf: &mut [u8]) -> usize {
        self.metadata().map_or(0, |metadata| {
            metadata
                .inode()
                .readat(metadata.offset(), buf)
                .map_or(0, |read| {
                    metadata.seek(read as i64, 1 /* SEEK_CURRENT */);
                    read
                })
        })
    }

    fn pread(&self, buf: &mut [u8], offset: u64) -> usize {
        self.metadata().map_or(0, |metadata| {
            metadata.inode().readat(offset as usize, buf).unwrap_or(0)
        })
    }

    fn pwrite(&self, buf: &[u8], offset: u64) -> usize {
        self.metadata().map_or(0, |metadata| {
            metadata.inode().writeat(offset as usize, buf).unwrap_or(0)
        })
    }
}

impl_downcast!(IFile);

pub struct FileDescriptorTable {
    table: Vec<Option<Arc<dyn IFile>>>,
    capacity: usize,
}

impl FileDescriptorTable {
    pub fn clone_for(&self) -> Self {
        let mut new = Self::new();

        for entry in self.table.iter().skip(3) {
            new.table.push(entry.clone());
        }

        debug_assert!(self.table.len() == new.table.len());

        new
    }

    pub fn clear_exec(&mut self) {
        for entry in self.table.iter_mut() {
            if let Some(file) = entry {
                if file.flags().contains(OpenFlags::O_CLOEXEC) {
                    *entry = None;
                }
            }
        }
    }
}

impl Default for FileDescriptorTable {
    fn default() -> Self {
        Self::new()
    }
}

impl FileDescriptorTable {
    /// Creates a new `FileDescriptorTable` with the given capacity.
    /// # Arguments
    /// * `task_id` - The ID of the task that owns the file descriptor table.
    pub fn new() -> Self {
        FileDescriptorTable {
            table: Vec::new(),
            capacity: Self::MAX_SIZE,
        }
    }

    /// Returns the file descriptor at the specified index.
    /// # Arguments
    /// * `idx` - The index of the file descriptor in the table.
    pub fn get(&self, idx: usize) -> Option<&Arc<dyn IFile>> {
        self.table.get(idx).and_then(|inner| inner.as_ref())
    }

    /// Sets the file descriptor at the specified index.
    /// # Arguments
    /// * `idx` - The index of the file descriptor in the table.
    /// * `fd` - The file descriptor to set.
    pub fn set(&mut self, idx: usize, fd: Arc<dyn IFile>) {
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
    pub fn allocate(&mut self, file: Arc<dyn IFile>) -> Option<usize> {
        for (idx, entry) in self.table.iter().enumerate() {
            if entry.is_none() {
                self.table[idx] = Some(file);
                return Some(idx);
            }
        }

        if self.table.len() >= self.capacity {
            return None;
        }

        self.table.push(Some(file));
        Some(self.table.len() - 1)
    }

    pub const MAX_SIZE: usize = 1024; // according to rlimit
    pub fn allocate_at(&mut self, file: Arc<dyn IFile>, idx: usize) -> Option<usize> {
        if idx >= self.capacity {
            return None;
        }

        if self.table.len() <= idx {
            self.table.reserve_exact(idx - self.table.len() + 1);
            self.table.resize_with(idx + 1, || None);
        } else if self.table.get(idx)?.is_some() {
            return None;
        }

        self.table[idx] = Some(file);
        Some(idx)
    }

    pub fn set_capacity(&mut self, new_capacity: usize) {
        self.capacity = new_capacity
    }

    pub fn get_capacity(&self) -> usize {
        self.capacity
    }
}

pub struct CachelessInodeFile {
    pub(crate) metadata: Arc<FileMetadata>,
}

impl IFile for CachelessInodeFile {
    fn metadata(&self) -> Option<Arc<FileMetadata>> {
        Some(self.metadata.clone())
    }
}

impl CachelessInodeFile {
    pub fn clear_type(self: Arc<Self>) -> Arc<dyn IFile> {
        self
    }
}
