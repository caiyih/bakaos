#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod fatfs_impl;

use alloc::{sync::Arc, vec::Vec};
pub use fatfs_impl::*;
use filesystem_abstractions::{IFileSystem, IInode};
use hermit_sync::{RawSpinMutex, SpinMutex};
use lock_api::{MappedMutexGuard, MutexGuard};
use log::debug;

pub type RootFileSystemType = fatfs_impl::Fat32FileSystem;

static mut ROOT_FILESYSTEM: Option<RootFileSystemType> = None;

pub fn setup_root_filesystem(fs: RootFileSystemType) {
    debug!("Initializing filesystem: {}", fs.name());

    unsafe { ROOT_FILESYSTEM = Some(fs) };
}

pub fn root_filesystem() -> &'static dyn IFileSystem {
    unsafe {
        ROOT_FILESYSTEM
            .as_ref()
            .expect("Root filesystem not initialized")
    }
}

static mut INODE_CACHE: SpinMutex<Vec<Arc<dyn IInode>>> = SpinMutex::new(Vec::new());

pub trait ICacheableInode: IInode {
    fn cache_as_arc_accessor(self: &Arc<Self>) -> Arc<InodeCacheAccessor>;
    fn cache_as_accessor(self: &Arc<Self>) -> InodeCacheAccessor;
}

impl ICacheableInode for dyn IInode {
    fn cache_as_accessor(self: &Arc<Self>) -> InodeCacheAccessor {
        InodeCacheAccessor::new(self.clone())
    }

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
        let inode_id = unsafe {
            let mut caches = INODE_CACHE.lock();
            let inode_id = caches.len();
            caches.push(inode);
            inode_id
        };

        InodeCacheAccessor { inode_id }
    }

    pub fn access(&self) -> Arc<dyn IInode> {
        let caches = unsafe { INODE_CACHE.lock() };
        let inode = caches[self.inode_id].clone();
        drop(caches); // prevent deadlock
        inode
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
    /// ```no_run
    /// use crate::filesystem::ICacheableInode;
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
    pub unsafe fn as_mut(&self) -> MappedMutexGuard<'static, RawSpinMutex, Arc<dyn IInode>> {
        let caches = INODE_CACHE.lock();
        MutexGuard::map(caches, |caches| &mut caches[self.inode_id])
    }
}

impl Drop for InodeCacheAccessor {
    fn drop(&mut self) {
        unsafe {
            INODE_CACHE.lock().swap_remove(self.inode_id);
        }
    }
}
