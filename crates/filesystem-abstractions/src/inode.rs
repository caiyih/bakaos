use alloc::collections::BTreeMap;
use alloc::string::String;
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

    fn read_cache_dir(
        &self,
        _caches: &mut BTreeMap<String, Arc<dyn IInode>>,
    ) -> FileSystemResult<Vec<DirectoryEntry>> {
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

    fn hard_link(&self, _name: &str, _inode: &Arc<dyn IInode>) -> FileSystemResult<()> {
        Err(FileSystemError::Unimplemented)
    }

    fn soft_link(&self, _name: &str, _point_to: &str) -> FileSystemResult<Arc<dyn IInode>> {
        Err(FileSystemError::Unimplemented)
    }

    fn resolve_link(&self) -> Option<String> {
        None
    }

    fn resize(&self, _new_size: u64) -> FileSystemResult<u64> {
        Err(FileSystemError::NotAFile)
    }

    fn removing(&self) -> FileSystemResult<()> {
        Ok(())
    }

    fn rename(&self, _old_name: &str, _new_name: &str) -> FileSystemResult<()> {
        Err(FileSystemError::Unimplemented)
    }

    fn renaming(&self, _new_name: &str) -> FileSystemResult<()> {
        Ok(())
    }
}

impl_downcast!(sync IInode);
