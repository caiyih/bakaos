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
use filesystem_abstractions::{
    global_mount, global_mount_filesystem, DirectoryEntryType, DirectoryTreeNode, IFileSystem,
    MountError,
};
// pub use lwext4rs_impl::Lwext4FileSystem;

pub fn global_mount_device(
    path: &str,
    mount_at: &str,
    relative_to: Option<&Arc<DirectoryTreeNode>>,
) -> Result<Arc<DirectoryTreeNode>, MountError> {
    let node = filesystem_abstractions::global_open(path, relative_to)
        .map_err(|_| MountError::FileNotExists)?;

    global_mount_device_node(&node, mount_at, relative_to)
}

pub fn global_mount_device_node(
    node: &Arc<DirectoryTreeNode>,
    path: &str,
    relative_to: Option<&Arc<DirectoryTreeNode>>,
) -> Result<Arc<DirectoryTreeNode>, MountError> {
    let metadata = node.metadata();

    match metadata.entry_type {
        DirectoryEntryType::Directory
        | DirectoryEntryType::Unknown
        | DirectoryEntryType::NamedPipe
        | DirectoryEntryType::CharDevice => global_mount(node, path, relative_to),
        _ => mount_block_device(node, path, relative_to),
    }
}

fn create_filesystem(device: Arc<DirectoryTreeNode>) -> Result<Arc<dyn IFileSystem>, MountError> {
    if let Ok(ext4) = Ext4FileSystem::new(device.clone()) {
        return Ok(Arc::new(ext4));
    }

    if let Ok(fat32) = Fat32FileSystem::new(device.clone()) {
        return Ok(Arc::new(fat32));
    }

    Err(MountError::InvalidInput) // Unsupported filesystem
}

fn mount_block_device(
    device: &Arc<DirectoryTreeNode>,
    path: &str,
    relative_to: Option<&Arc<DirectoryTreeNode>>,
) -> Result<Arc<DirectoryTreeNode>, MountError> {
    let fs = create_filesystem(device.clone())?;

    global_mount_filesystem(fs, path, relative_to)
}
