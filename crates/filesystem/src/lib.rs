#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod fatfs_impl;

use alloc::sync::Arc;

pub use fatfs_impl::*;
use filesystem_abstractions::{mount_at, IFileSystem};

pub type RootFileSystemType = fatfs_impl::Fat32FileSystem;

pub fn setup_root_filesystem(fs: RootFileSystemType) {
    let root: Arc<dyn IFileSystem> = Arc::new(fs);
    mount_at(root, "/");
}
