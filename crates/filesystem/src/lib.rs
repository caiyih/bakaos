#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod fatfs_impl;

pub use fatfs_impl::*;
use filesystem_abstractions::IFileSystem;
use log::debug;

pub type RootFileSystemType = fatfs_impl::Fat32FileSystem;

static mut ROOT_FILESYSTEM: Option<RootFileSystemType> = None;

pub fn setup_root_filesystem(fs: RootFileSystemType) {
    debug!("Initializing filesystem: {}", fs.name());

    unsafe { ROOT_FILESYSTEM = Some(fs) };
}

pub fn root_filesystem() -> &'static RootFileSystemType {
    unsafe {
        ROOT_FILESYSTEM
            .as_ref()
            .expect("Root filesystem not initialized")
    }
}
