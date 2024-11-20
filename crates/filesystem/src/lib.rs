#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod fatfs_impl;

use alloc::{sync::Arc, vec::Vec};
pub use fatfs_impl::*;
use filesystem_abstractions::{IFileSystem, IInode};
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

static mut INODE_CACHE: Vec<Arc<dyn IInode>> = Vec::new();

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
            let inode_id = INODE_CACHE.len();
            INODE_CACHE.push(inode);
            inode_id
        };

        InodeCacheAccessor { inode_id }
    }

    pub fn access(&self) -> Arc<dyn IInode> {
        unsafe { INODE_CACHE[self.inode_id].clone() }
    }

    pub fn inode_id(&self) -> usize {
        self.inode_id
    }
}

impl Drop for InodeCacheAccessor {
    fn drop(&mut self) {
        unsafe {
            INODE_CACHE.swap_remove(self.inode_id);
        }
    }
}
