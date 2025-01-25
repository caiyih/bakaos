#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod ext4_impl;
mod fatfs_impl;
// mod lwext4rs_impl;

use alloc::sync::Arc;

pub use ext4_impl::Ext4FileSystem;
pub use fatfs_impl::Fat32FileSystem;
use filesystem_abstractions::{IFileSystem, IInode};
// pub use lwext4rs_impl::Lwext4FileSystem;

pub struct DummyInode;

impl IInode for DummyInode {}

pub struct DummyFileSystem;

impl Default for DummyFileSystem {
    fn default() -> Self {
        DummyFileSystem
    }
}

impl IFileSystem for DummyFileSystem {
    fn root_dir(&self) -> Arc<dyn IInode> {
        Arc::new(DummyInode)
    }

    fn name(&self) -> &str {
        "DummyFileSystem"
    }
}
