#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod ext4_impl;
mod fatfs_impl;
// mod lwext4rs_impl;

use alloc::{boxed::Box, sync::Arc};

pub use ext4_impl::Ext4FileSystem;
pub use fatfs_impl::Fat32FileSystem;
use filesystem_abstractions::{
    global_mount, DirectoryEntryType, DirectoryTreeNode, IFileSystem, IInode, MountError,
};
// pub use lwext4rs_impl::Lwext4FileSystem;

pub fn global_mount_device(
    path: &str,
    mount_at: &str,
    relative_to: Option<&Arc<DirectoryTreeNode>>,
) -> Result<Arc<DirectoryTreeNode>, MountError> {
    let inode: Arc<dyn IInode> = filesystem_abstractions::global_open(path, relative_to)
        .map_err(|_| MountError::FileNotExists)?;

    global_mount_device_inode(&inode, mount_at, relative_to)
}

pub fn global_mount_device_inode(
    inode: &Arc<dyn IInode>,
    path: &str,
    relative_to: Option<&Arc<DirectoryTreeNode>>,
) -> Result<Arc<DirectoryTreeNode>, MountError> {
    let metadata = inode.metadata();

    match metadata.entry_type {
        DirectoryEntryType::BlockDevice => mount_block_device(inode, path, relative_to),
        _ => global_mount(inode, path, relative_to),
    }
}

fn create_filesystem(device: Arc<dyn IInode>) -> Result<Box<dyn IFileSystem>, MountError> {
    if let Ok(ext4) = Ext4FileSystem::new(device.clone()) {
        return Ok(Box::new(ext4));
    }

    if let Ok(fat32) = Fat32FileSystem::new(device.clone()) {
        return Ok(Box::new(fat32));
    }

    Err(MountError::InvalidInput) // Unsupported filesystem
}

fn mount_block_device(
    device: &Arc<dyn IInode>,
    path: &str,
    relative_to: Option<&Arc<DirectoryTreeNode>>,
) -> Result<Arc<DirectoryTreeNode>, MountError> {
    let fs = create_filesystem(device.clone())?;

    global_mount(&fs.root_dir(), path, relative_to)
}
