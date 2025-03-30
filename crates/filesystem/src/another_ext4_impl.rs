use alloc::{string::String, sync::Arc, vec::Vec};
use another_ext4::{
    ErrCode, Ext4, Ext4Error, FileType, InodeMode, PBlockId, BLOCK_SIZE, EXT4_ROOT_INO,
};
use filesystem_abstractions::{
    DirectoryEntry, DirectoryEntryType, DirectoryTreeNode, FileStatisticsMode, FileSystemError,
    IFileSystem, IInode, InodeMetadata,
};
use timing::TimeSpec;

struct BlockDeviceNode {
    device: Arc<DirectoryTreeNode>,
}

impl another_ext4::BlockDevice for BlockDeviceNode {
    fn read_block(&self, block_id: PBlockId) -> another_ext4::Block {
        let mut block = another_ext4::Block::new(block_id, [0; BLOCK_SIZE]);

        let _ = self
            .device
            .readat((block_id * BLOCK_SIZE as u64) as usize, &mut block.data);

        block
    }

    fn write_block(&self, block: &another_ext4::Block) {
        let _ = self
            .device
            .writeat((block.id * BLOCK_SIZE as u64) as usize, &block.data);
    }
}

pub struct AnotherExt4FileSystem {
    root: Arc<AnotherExt4Inode>,
}

fn to_filesystem_error(e: Ext4Error) -> FileSystemError {
    log::warn!("AnotherExt4FileSystem: {:?}", e);

    match e.code() {
        ErrCode::ENOENT => FileSystemError::NotFound,
        ErrCode::EIO => FileSystemError::InternalError,
        ErrCode::ENXIO => FileSystemError::InternalError,
        ErrCode::E2BIG => FileSystemError::InvalidInput,
        ErrCode::EEXIST => FileSystemError::AlreadyExists,
        ErrCode::ENOTDIR => FileSystemError::NotADirectory,
        ErrCode::EISDIR => FileSystemError::NotAFile,
        ErrCode::EINVAL => FileSystemError::InvalidInput,
        ErrCode::EFBIG => FileSystemError::SpaceNotEnough,
        ErrCode::ENOSPC => FileSystemError::SpaceNotEnough,
        ErrCode::EMLINK => FileSystemError::LinkTooDepth,
        ErrCode::ERANGE => FileSystemError::InvalidInput,
        ErrCode::ENOTEMPTY => FileSystemError::DirectoryNotEmpty,
        ErrCode::ENODATA => FileSystemError::InternalError,
        ErrCode::ELINKFAIL => FileSystemError::LinkTooDepth,
        ErrCode::EALLOCFAIL => FileSystemError::SpaceNotEnough,
        _ => FileSystemError::Unimplemented,
    }
}

impl AnotherExt4FileSystem {
    #[allow(clippy::result_unit_err)]
    pub fn new(device: Arc<DirectoryTreeNode>) -> Result<AnotherExt4FileSystem, ()> {
        let mut inner = another_ext4::Ext4::load(Arc::new(BlockDeviceNode { device }))
            .map_err(|e| log::warn!("Failed to parse node as ext4: {:?}", e))?;

        inner
            .init()
            .map_err(|e| log::warn!("Failed to init ext4: {:?}", e))?;

        Ok(AnotherExt4FileSystem {
            root: Arc::new(AnotherExt4Inode {
                fs: Arc::new(inner),
                inode: EXT4_ROOT_INO,
                file_type: DirectoryEntryType::Directory,
                filename: String::new(),
            }),
        })
    }
}

impl IFileSystem for AnotherExt4FileSystem {
    fn root_dir(&self) -> alloc::sync::Arc<dyn filesystem_abstractions::IInode> {
        self.root.clone()
    }

    fn name(&self) -> &str {
        "AnotherExt4FileSystem"
    }
}

struct AnotherExt4Inode {
    fs: Arc<Ext4>,
    inode: another_ext4::InodeId,
    file_type: DirectoryEntryType, // TODO: make this updatable, there may be cases where the file was removed and created with different type while the inode is still valid in memory
    filename: String,
}

impl AnotherExt4Inode {
    fn should_be_directory(&self) -> Result<(), FileSystemError> {
        if self.file_type != DirectoryEntryType::Directory {
            return Err(FileSystemError::NotADirectory);
        }

        Ok(())
    }

    fn should_be_file(&self) -> Result<(), FileSystemError> {
        if self.file_type != DirectoryEntryType::File {
            return Err(FileSystemError::NotAFile);
        }

        Ok(())
    }

    fn should_be_link(&self) -> Result<(), FileSystemError> {
        if self.file_type != DirectoryEntryType::Symlink {
            return Err(FileSystemError::NotALink);
        }

        Ok(())
    }
}

impl IInode for AnotherExt4Inode {
    fn metadata(&self) -> filesystem_abstractions::InodeMetadata {
        let size = match self.file_type {
            DirectoryEntryType::File => self.fs.getattr(self.inode).map(|a| a.size).unwrap_or(0),
            DirectoryEntryType::BlockDevice => 0, // TODO: What to do with block devices? We do not allow them in the ext4 filesystem
            _ => 0,
        };

        InodeMetadata {
            filename: &self.filename,
            entry_type: self.file_type,
            size: size as usize,
        }
    }

    fn flush(&self) -> filesystem_abstractions::FileSystemResult<()> {
        self.fs.flush_all();

        Ok(())
    }

    fn lookup(&self, name: &str) -> filesystem_abstractions::FileSystemResult<Arc<dyn IInode>> {
        self.should_be_directory()?;

        let inode = self
            .fs
            .lookup(self.inode, name)
            .map_err(to_filesystem_error)?;

        let attr = self.fs.getattr(inode).map_err(to_filesystem_error)?;

        Ok(Arc::new(AnotherExt4Inode {
            fs: self.fs.clone(),
            inode,
            file_type: DirectoryEntryType::from_ext4(attr.ftype),
            filename: String::from(name),
        }))
    }

    fn mkdir(&self, name: &str) -> filesystem_abstractions::FileSystemResult<Arc<dyn IInode>> {
        self.should_be_directory()?;

        let inode = self
            .fs
            .mkdir(self.inode, name, INODE_MODE_ALL | InodeMode::DIRECTORY)
            .map_err(to_filesystem_error)?;

        Ok(Arc::new(AnotherExt4Inode {
            fs: self.fs.clone(),
            inode,
            file_type: DirectoryEntryType::Directory,
            filename: String::from(name),
        }))
    }

    fn remove(&self, name: &str) -> filesystem_abstractions::FileSystemResult<()> {
        self.should_be_directory()?;

        self.fs
            .unlink(self.inode, name)
            .map_err(to_filesystem_error)
    }

    fn rmdir(&self, name: &str) -> filesystem_abstractions::FileSystemResult<()> {
        self.should_be_directory()?;

        self.fs.rmdir(self.inode, name).map_err(to_filesystem_error)
    }

    fn touch(&self, name: &str) -> filesystem_abstractions::FileSystemResult<Arc<dyn IInode>> {
        self.should_be_directory()?;

        let inode = self
            .fs
            .create(self.inode, name, INODE_MODE_ALL | InodeMode::FILE)
            .map_err(to_filesystem_error)?;

        Ok(Arc::new(AnotherExt4Inode {
            fs: self.fs.clone(),
            inode,
            file_type: DirectoryEntryType::File,
            filename: String::from(name),
        }))
    }

    fn read_cache_dir(
        &self,
        // TODO: investigate if we need to use caches
        _caches: &mut alloc::collections::btree_map::BTreeMap<String, Arc<dyn IInode>>,
    ) -> filesystem_abstractions::FileSystemResult<
        alloc::vec::Vec<filesystem_abstractions::DirectoryEntry>,
    > {
        self.should_be_directory()?;

        let mut raw_entries = self.fs.listdir(self.inode).map_err(to_filesystem_error)?;

        let heap_warn = raw_entries.len() > 1000;

        let mut entries = if heap_warn {
            Vec::with_capacity(raw_entries.len())
        } else {
            Vec::with_capacity(1000)
        };

        while let Some(ext4_entry) = raw_entries.pop() {
            if ext4_entry.compare_name(path::CURRENT_DIRECTORY)
                || ext4_entry.compare_name(path::PARENT_DIRECTORY)
            {
                continue;
            }

            if entries.len() == entries.capacity() {
                if entries.capacity() < raw_entries.len() + 1 {
                    entries.reserve(entries.capacity());
                } else {
                    entries.reserve_exact(raw_entries.len() + 1);
                }
            }

            entries.push(DirectoryEntry {
                filename: ext4_entry.name(),
                entry_type: DirectoryEntryType::from_ext4(ext4_entry.file_type()),
            });

            // Release memory to reduce memory usage
            if heap_warn && raw_entries.len() % 100 == 0 {
                raw_entries.shrink_to_fit();
            }
        }

        Ok(entries)
    }

    fn hard_link(
        &self,
        name: &str,
        inode: &Arc<dyn IInode>,
    ) -> filesystem_abstractions::FileSystemResult<()> {
        if let Some(node) = inode.downcast_ref::<AnotherExt4Inode>() {
            if Arc::ptr_eq(&self.fs, &node.fs) {
                self.fs
                    .link(node.inode, self.inode, name)
                    .map_err(to_filesystem_error)?;
            }
        }

        Err(FileSystemError::InvalidInput)
    }

    fn stat(
        &self,
        stat: &mut filesystem_abstractions::FileStatistics,
    ) -> filesystem_abstractions::FileSystemResult<()> {
        let attr = self.fs.getattr(self.inode).map_err(to_filesystem_error)?;

        // This is not needed for correctness, but it is needed for testing
        // This is only used to observe if we should make self.file_type be updated
        debug_assert_eq!(self.file_type, DirectoryEntryType::from_ext4(attr.ftype));

        stat.size = attr.size;
        stat.inode_id = attr.ino as u64;
        // stat.rdev
        stat.mode = FileStatisticsMode::OWNER_MASK
            | FileStatisticsMode::GROUP_MASK
            | FileStatisticsMode::OTHER_MASK
            | match attr.ftype {
                FileType::Unknown => FileStatisticsMode::NULL,
                FileType::RegularFile => FileStatisticsMode::FILE,
                FileType::Directory => FileStatisticsMode::DIR,
                FileType::CharacterDev => FileStatisticsMode::CHAR,
                FileType::BlockDev => FileStatisticsMode::BLOCK,
                FileType::Fifo => FileStatisticsMode::FIFO,
                FileType::Socket => FileStatisticsMode::SOCKET,
                FileType::SymLink => FileStatisticsMode::LINK,
            };

        stat.device_id = 1;
        stat.link_count = 1;
        stat.uid = 0;
        stat.gid = 0;
        stat.block_size = BLOCK_SIZE as u32;
        stat.block_count = attr.blocks;
        stat.atime = TimeSpec {
            tv_sec: attr.atime as i64,
            tv_nsec: 0,
        };

        stat.mtime = TimeSpec {
            tv_sec: attr.mtime as i64,
            tv_nsec: 0,
        };
        stat.ctime = TimeSpec {
            tv_sec: attr.ctime as i64,
            tv_nsec: 0,
        };

        Ok(())
    }

    fn resolve_link(&self) -> Option<String> {
        self.should_be_link().ok()?;

        let mut buf = [0u8; 1024];
        let read = self
            .fs
            .read(self.inode, 0, &mut buf)
            .map_err(to_filesystem_error)
            .ok()?;

        let link = core::str::from_utf8(&buf[..read])
            .map_err(|e| {
                log::warn!(
                    "Unable to parse link, error: {:?}. Content: {:?}",
                    e,
                    &buf[..read]
                )
            })
            .ok()?;

        Some(String::from(link))
    }

    fn soft_link(
        &self,
        name: &str,
        point_to: &str,
    ) -> filesystem_abstractions::FileSystemResult<Arc<dyn IInode>> {
        self.should_be_directory()?;

        let inode = self
            .fs
            .create(self.inode, name, INODE_MODE_ALL | InodeMode::SOFTLINK)
            .map_err(to_filesystem_error)?;

        self.fs
            .write(inode, 0, point_to.as_bytes())
            .map_err(to_filesystem_error)?;

        Ok(Arc::new(AnotherExt4Inode {
            fs: self.fs.clone(),
            inode,
            file_type: DirectoryEntryType::Symlink,
            filename: String::from(name),
        }))
    }

    fn resize(&self, new_size: u64) -> filesystem_abstractions::FileSystemResult<u64> {
        self.should_be_file()?;

        self.fs
            .setattr(
                self.inode,
                None,
                None,
                None,
                Some(new_size),
                None,
                None,
                None,
                None,
            )
            .map_err(to_filesystem_error)?;

        Ok(new_size)
    }

    fn readat(
        &self,
        offset: usize,
        buffer: &mut [u8],
    ) -> filesystem_abstractions::FileSystemResult<usize> {
        self.should_be_file()?;

        self.fs
            .read(self.inode, offset, buffer)
            .map_err(to_filesystem_error)
    }

    fn writeat(
        &self,
        offset: usize,
        buffer: &[u8],
    ) -> filesystem_abstractions::FileSystemResult<usize> {
        self.should_be_file()?;

        self.fs
            .write(self.inode, offset, buffer)
            .map_err(to_filesystem_error)
    }
}

const INODE_MODE_ALL: InodeMode = InodeMode::USER_READ
    .union(InodeMode::USER_WRITE)
    .union(InodeMode::USER_EXEC)
    .union(InodeMode::GROUP_READ)
    .union(InodeMode::GROUP_WRITE)
    .union(InodeMode::GROUP_EXEC)
    .union(InodeMode::OTHER_READ)
    .union(InodeMode::OTHER_WRITE)
    .union(InodeMode::OTHER_EXEC);

trait IFileType<T> {
    fn from_ext4(t: T) -> Self;
}

impl IFileType<FileType> for DirectoryEntryType {
    fn from_ext4(t: FileType) -> Self {
        match t {
            FileType::RegularFile => DirectoryEntryType::File,
            FileType::Directory => DirectoryEntryType::Directory,
            FileType::CharacterDev => DirectoryEntryType::CharDevice,
            FileType::BlockDev => DirectoryEntryType::BlockDevice,
            FileType::Fifo => DirectoryEntryType::NamedPipe,
            FileType::SymLink => DirectoryEntryType::Symlink,
            _ => DirectoryEntryType::Unknown,
        }
    }
}
