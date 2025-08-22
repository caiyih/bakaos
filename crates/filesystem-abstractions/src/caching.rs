use core::{
    ops::Deref,
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::sync::Arc;
use alloc::vec::Vec;
use hermit_sync::{RawSpinMutex, SpinMutex, SpinMutexGuard};
use lock_api::{MappedMutexGuard, MutexGuard};

use crate::IFile;

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

static FILE_TABLE: SpinMutex<Vec<Option<FileCacheEntry>>> = SpinMutex::new(Vec::new());

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
        let caches = FILE_TABLE.lock();

        caches[file_id].as_ref()?.add_reference();

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

impl Clone for FileCacheAccessor {
    fn clone(&self) -> Self {
        Self::new(self.file_id).unwrap()
    }
}

impl FileCacheAccessor {
    fn cache(file: Arc<dyn IFile>) -> FileCacheAccessor {
        let mut caches = FILE_TABLE.lock();

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
        let caches = FILE_TABLE.lock();
        let entry = caches[self.file_id]
            .as_ref()
            .expect("Entry should still exist as this accessor still holds a reference.");

        // at least *this* accessor should have a reference to the file.
        debug_assert!(!entry.is_zombie());

        entry.cahce.clone()
    }

    pub fn access_ref(&self) -> MappedMutexGuard<RawSpinMutex, Arc<dyn IFile>> {
        SpinMutexGuard::map(FILE_TABLE.lock(), |c| {
            &mut c[self.file_id].as_mut().unwrap().cahce
        })
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
