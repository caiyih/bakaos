#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod ext4_impl;
mod fatfs_impl;

use alloc::sync::Arc;

pub use ext4_impl::Ext4FileSystem;
pub use fatfs_impl::Fat32FileSystem;
use filesystem_abstractions::{mount_at, IFileSystem, IInode};

pub type RootFileSystemType = fatfs_impl::Fat32FileSystem;

pub fn setup_root_filesystem(fs: RootFileSystemType) {
    let root: Arc<dyn IFileSystem> = Arc::new(fs);
    mount_at(root, "/");
}

pub struct DummyInode;

impl IInode for DummyInode {}

pub struct DummyFileSystem;

impl Default for DummyFileSystem {
    fn default() -> Self {
        DummyFileSystem
    }
}

impl IFileSystem for DummyFileSystem {
    fn root_dir(&'static self) -> Arc<dyn IInode> {
        Arc::new(DummyInode)
    }

    fn name(&self) -> &str {
        "DummyFileSystem"
    }
}
