use core::{
    ops::Deref,
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::sync::Arc;
use alloc::vec::Vec;
use hermit_sync::{RawSpinMutex, SpinMutex};
use lock_api::{MappedMutexGuard, MutexGuard};

use crate::{IFile, IInode};

pub struct FileCacheEntry {
    pub cahce: Arc<dyn IFile>,
    pub rc: AtomicUsize,
}

impl FileCacheEntry {
    pub fn add_reference(&self) {
        self.rc.fetch_add(1, Ordering::Relaxed);
    }

    pub fn remove_reference(&self) {
        self.rc.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn is_zombie(&self) -> bool {
        self.rc.load(Ordering::Relaxed) == 0
    }

    pub fn references(&self) -> usize {
        self.rc.load(Ordering::Relaxed)
    }
}

static mut FILE_TABLE: SpinMutex<Vec<Option<FileCacheEntry>>> = SpinMutex::new(Vec::new());

pub trait ICacheableFile: IFile {
    fn cache_as_arc_accessor(self: &Arc<Self>) -> Arc<FileCacheAccessor> {
        Arc::new(self.cache_as_accessor())
    }

    fn cache_as_accessor(self: &Arc<Self>) -> FileCacheAccessor;
}

impl ICacheableFile for dyn IFile {
    fn cache_as_accessor(self: &Arc<Self>) -> FileCacheAccessor {
        FileCacheAccessor::cache(self.clone())
    }
}

#[derive(Debug)]
pub struct FileCacheAccessor {
    file_id: usize,
}

impl FileCacheAccessor {
    pub fn table_idx(&self) -> usize {
        self.file_id
    }

    fn new(file_id: usize) -> Option<Self> {
        unsafe {
            let caches = FILE_TABLE.lock();

            caches[file_id].as_ref()?.add_reference();
        }

        Some(Self { file_id })
    }
}

impl FileCacheAccessor {
    pub fn clone_non_inherited_arc(self: &Arc<Self>) -> Arc<Self> {
        Arc::new(self.deref().clone())
    }
}

impl Drop for FileCacheAccessor {
    fn drop(&mut self) {
        unsafe {
            let mut caches = FILE_TABLE.lock();

            let entry = caches[self.file_id]
                .as_ref()
                .expect("Entry should still exist as this accessor still holds a reference.");

            // Remove close rc added by *this* accessor.
            // Rc should have been removed if the accessor is closed.
            entry.remove_reference();

            // Clear the cache entry if the file is closed and there are no references to it.
            if entry.is_zombie() {
                caches[self.file_id] = None;
            }
        }
    }
}

impl Clone for FileCacheAccessor {
    fn clone(&self) -> Self {
        Self::new(self.file_id).unwrap()
    }
}

impl FileCacheAccessor {
    fn cache(file: Arc<dyn IFile>) -> FileCacheAccessor {
        let mut caches = unsafe { FILE_TABLE.lock() };

        let file_id = match caches.iter().enumerate().find(|x| x.1.is_none()) {
            Some((index, _)) => {
                caches[index] = Some(FileCacheEntry {
                    cahce: file.clone(),
                    rc: AtomicUsize::new(0),
                });
                index
            }
            None => {
                caches.push(Some(FileCacheEntry {
                    cahce: file.clone(),
                    rc: AtomicUsize::new(0),
                }));
                caches.len() - 1
            }
        };

        drop(caches); // `new` method requires the lock.
                      // So we need to drop the lock to prevent deadlock.

        FileCacheAccessor::new(file_id).unwrap()
    }

    pub fn access(&self) -> Arc<dyn IFile> {
        let caches = unsafe { FILE_TABLE.lock() };
        let entry = caches[self.file_id]
            .as_ref()
            .expect("Entry should still exist as this accessor still holds a reference.");

        // at least *this* accessor should have a reference to the file.
        debug_assert!(!entry.is_zombie());

        entry.cahce.clone()
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
    pub unsafe fn access_cache_entry(
        &self,
    ) -> MappedMutexGuard<'static, RawSpinMutex, FileCacheEntry> {
        let caches = FILE_TABLE.lock();
        MutexGuard::map(caches, |caches| caches[self.file_id].as_mut().unwrap())
    }
}

static mut INODE_CACHE: SpinMutex<Vec<Option<Arc<dyn IInode>>>> = SpinMutex::new(Vec::new());

pub trait ICacheableInode: IInode {
    fn cache_as_arc_accessor(self: &Arc<Self>) -> Arc<InodeCacheAccessor>;
    fn cache_as_accessor(self: &Arc<Self>) -> InodeCacheAccessor;
}

impl ICacheableInode for dyn IInode {
    /// Cache the inode in the kernel's inode table and returns an accessor to the inode.
    /// The accessor wraps the index of the inode in the inode table and is the only way to access the inode.
    /// The inode will be removed from the inode table when the accessor is dropped.
    fn cache_as_accessor(self: &Arc<Self>) -> InodeCacheAccessor {
        InodeCacheAccessor::new(self.clone())
    }

    /// Similar to `cache_as_accessor`, but returns an Arc of the accessor.
    /// See `cache_as_accessor` for more information.
    fn cache_as_arc_accessor(self: &Arc<Self>) -> Arc<InodeCacheAccessor> {
        Arc::new(self.cache_as_accessor())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct InodeCacheAccessor {
    inode_id: usize,
}

impl InodeCacheAccessor {
    pub fn new(inode: Arc<dyn IInode>) -> Self {
        let mut caches = unsafe { INODE_CACHE.lock() };

        let inode_id = match caches.iter().enumerate().find(|x| x.1.is_none()) {
            // Reuse the index of the first None element
            Some((index, _)) => {
                caches[index] = Some(inode);
                index
            }
            // Add a new element to the end of the vector
            None => {
                caches.push(Some(inode));
                caches.len() - 1
            }
        };

        InodeCacheAccessor { inode_id }
    }

    // Access the inode from the inode cache table provided by the accessor.
    pub fn access(&self) -> Arc<dyn IInode> {
        let caches = unsafe { INODE_CACHE.lock() };
        let inode = caches[self.inode_id].clone();
        drop(caches); // prevent deadlock
        inode.unwrap() // unwrap is safe because as long as the accessor exists, the inode will exist
                       // Same appies to the unwrap call in the as_mut method
    }

    pub fn inode_id(&self) -> usize {
        self.inode_id
    }

    /// Access the mutable element of the inode cache table.
    /// # Safety
    /// I made this method unsafe because you can change the value of the inode cache table.
    /// This can be a dangerous operation, so you need to be very careful when using this method.
    ///
    /// # Example
    /// ```ignore
    /// use filesystem_abstractions::ICacheableInode;
    ///
    /// let text_cache = filesystem::root_filesystem()
    ///    .lookup("/text.txt")
    ///    .expect("text.txt not found")
    ///    .cache_as_accessor();
    ///
    /// let mut pInode = unsafe { text_cache.as_mut() };
    ///
    /// // Update the inode cache with another inode
    ///  *pInode = filesystem::root_filesystem()
    ///     .lookup("/new_text.txt")
    ///     .expect("new_text.txt not found");
    ///
    /// // The cache accessor returns the new inode
    /// let new_text = text_cache.access();
    /// ```
    pub unsafe fn access_mut(&self) -> MappedMutexGuard<'static, RawSpinMutex, Arc<dyn IInode>> {
        let caches = INODE_CACHE.lock();
        MutexGuard::map(caches, |caches| caches[self.inode_id].as_mut().unwrap())
    }
}

impl Drop for InodeCacheAccessor {
    // Remove the inode from the inode cache table when the accessor is dropped.
    fn drop(&mut self) {
        unsafe {
            INODE_CACHE.lock()[self.inode_id] = None;
        }
    }
}
