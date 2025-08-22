use alloc::sync::Arc;
use timing::TimeSpec;

use crate::{DirectoryEntryType, FileStatisticsMode, FileSystemResult, IInode, InodeMetadata};

pub struct NullInode;

impl NullInode {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Arc<dyn IInode> {
        Arc::new(NullInode)
    }
}

impl IInode for NullInode {
    fn metadata(&self) -> InodeMetadata {
        InodeMetadata {
            filename: "null",
            entry_type: DirectoryEntryType::CharDevice,
            size: 0,
        }
    }

    fn stat(&self, stat: &mut crate::FileStatistics) -> FileSystemResult<()> {
        stat.device_id = 0;
        stat.inode_id = 0;
        stat.mode = FileStatisticsMode::CHAR;
        stat.link_count = 1;
        stat.uid = 0;
        stat.gid = 0;
        stat.size = 0;
        stat.block_size = 512;
        stat.block_count = 0;
        stat.rdev = 0;

        stat.ctime = TimeSpec::zero();
        stat.mtime = TimeSpec::zero();
        stat.atime = TimeSpec::zero();

        Ok(())
    }

    fn readat(&self, _offset: usize, _buffer: &mut [u8]) -> FileSystemResult<usize> {
        Ok(0)
    }

    fn writeat(&self, _offset: usize, buffer: &[u8]) -> FileSystemResult<usize> {
        Ok(buffer.len())
    }
}

pub struct ZeroInode;

impl ZeroInode {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Arc<dyn IInode> {
        Arc::new(ZeroInode)
    }
}

impl IInode for ZeroInode {
    fn metadata(&self) -> InodeMetadata {
        InodeMetadata {
            filename: "zero",
            entry_type: DirectoryEntryType::CharDevice,
            size: 0,
        }
    }

    fn stat(&self, stat: &mut crate::FileStatistics) -> FileSystemResult<()> {
        stat.device_id = 0;
        stat.inode_id = 0;
        stat.mode = FileStatisticsMode::CHAR;
        stat.link_count = 1;
        stat.uid = 0;
        stat.gid = 0;
        stat.size = 0;
        stat.block_size = 512;
        stat.block_count = 0;
        stat.rdev = 0;

        stat.ctime = TimeSpec::zero();
        stat.mtime = TimeSpec::zero();
        stat.atime = TimeSpec::zero();

        Ok(())
    }

    fn readat(&self, _offset: usize, buffer: &mut [u8]) -> FileSystemResult<usize> {
        buffer.fill(0);

        Ok(buffer.len())
    }

    fn writeat(&self, _offset: usize, buffer: &[u8]) -> FileSystemResult<usize> {
        Ok(buffer.len())
    }
}

pub struct RandomInode;

impl RandomInode {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Arc<dyn IInode> {
        Arc::new(RandomInode)
    }
}

impl IInode for RandomInode {
    fn metadata(&self) -> InodeMetadata {
        InodeMetadata {
            filename: "random",
            entry_type: DirectoryEntryType::CharDevice,
            size: 0,
        }
    }

    fn stat(&self, stat: &mut crate::FileStatistics) -> FileSystemResult<()> {
        stat.device_id = 0;
        stat.inode_id = 0;
        stat.mode = FileStatisticsMode::CHAR;
        stat.link_count = 1;
        stat.uid = 0;
        stat.gid = 0;
        stat.size = 0;
        stat.block_size = 512;
        stat.block_count = 0;
        stat.rdev = 0;

        stat.ctime = TimeSpec::zero();
        stat.mtime = TimeSpec::zero();
        stat.atime = TimeSpec::zero();

        Ok(())
    }

    fn readat(&self, _offset: usize, buffer: &mut [u8]) -> FileSystemResult<usize> {
        rng::global_fill_safe(buffer);

        Ok(buffer.len())
    }

    fn writeat(&self, _offset: usize, buffer: &[u8]) -> FileSystemResult<usize> {
        Ok(buffer.len())
    }
}

pub struct UnblockedRandomInode;

impl UnblockedRandomInode {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Arc<dyn IInode> {
        Arc::new(UnblockedRandomInode)
    }
}

impl IInode for UnblockedRandomInode {
    fn metadata(&self) -> InodeMetadata {
        InodeMetadata {
            filename: "urandom",
            entry_type: DirectoryEntryType::CharDevice,
            size: 0,
        }
    }

    fn stat(&self, stat: &mut crate::FileStatistics) -> FileSystemResult<()> {
        stat.device_id = 0;
        stat.inode_id = 0;
        stat.mode = FileStatisticsMode::CHAR;
        stat.link_count = 1;
        stat.uid = 0;
        stat.gid = 0;
        stat.size = 0;
        stat.block_size = 512;
        stat.block_count = 0;
        stat.rdev = 0;

        stat.ctime = TimeSpec::zero();
        stat.mtime = TimeSpec::zero();
        stat.atime = TimeSpec::zero();

        Ok(())
    }

    fn readat(&self, _offset: usize, buffer: &mut [u8]) -> FileSystemResult<usize> {
        rng::global_fill_fast(buffer);

        Ok(buffer.len())
    }

    fn writeat(&self, _offset: usize, buffer: &[u8]) -> FileSystemResult<usize> {
        Ok(buffer.len())
    }
}
