use alloc::{sync::Arc, vec::Vec};
use downcast_rs::{impl_downcast, DowncastSync};

use crate::{DirectoryEntry, FileStatistics, FileSystemError, FileSystemResult, InodeMetadata};

pub trait IInode: DowncastSync + Send + Sync {
    fn metadata(&self) -> InodeMetadata;

    fn readat(&self, _offset: usize, _buffer: &mut [u8]) -> FileSystemResult<usize> {
        Err(FileSystemError::NotAFile)
    }

    fn writeat(&self, _offset: usize, _buffer: &[u8]) -> FileSystemResult<usize> {
        Err(FileSystemError::NotAFile)
    }

    fn mkdir(&self, _name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        Err(FileSystemError::NotADirectory)
    }

    fn rmdir(&self, _name: &str) -> FileSystemResult<()> {
        Err(FileSystemError::NotADirectory)
    }

    fn remove(&self, _name: &str) -> FileSystemResult<()> {
        Err(FileSystemError::NotADirectory)
    }

    fn touch(&self, _name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        Err(FileSystemError::NotADirectory)
    }

    fn read_dir(&self) -> FileSystemResult<Vec<DirectoryEntry>> {
        Err(FileSystemError::NotADirectory)
    }

    #[deprecated = "This is an internal method, use global_open or DirectoryTreeNode::open"]
    fn lookup(&self, _name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        Err(FileSystemError::NotADirectory)
    }

    fn flush(&self) -> FileSystemResult<()> {
        Err(FileSystemError::Unimplemented)
    }

    fn stat(&self, _stat: &mut FileStatistics) -> FileSystemResult<()> {
        Err(FileSystemError::Unimplemented)
    }
}

impl_downcast!(sync IInode);
