#![allow(clippy::arc_with_non_send_sync)]

use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::{ops::Deref, str};

use fatfs::{Dir, Error, File, LossyOemCpConverter, NullTimeProvider, Read, Seek, SeekFrom, Write};
use filesystem_abstractions::{
    DirectoryEntryType, DirectoryTreeNode, FileStatistics, FileStatisticsMode, FileSystemError,
    FileSystemResult, IFileSystem, IInode, InodeMetadata,
};
use hermit_sync::SpinMutex;
use log::warn;

const FILESYSTEM_NAME: &str = "Fat32FileSystem";

pub struct Fat32FileSystem {
    root_dir: Arc<dyn IInode>,
}

unsafe impl Send for Fat32FileSystem {}
unsafe impl Sync for Fat32FileSystem {}

pub struct Fat32Disk {
    inner: SpinMutex<(Arc<DirectoryTreeNode>, u64)>,
}

impl IFileSystem for Fat32FileSystem {
    fn root_dir(&self) -> alloc::sync::Arc<dyn filesystem_abstractions::IInode> {
        self.root_dir.clone()
    }

    fn name(&self) -> &str {
        FILESYSTEM_NAME
    }
}

impl Deref for Fat32FileSystem {
    type Target = Arc<dyn IInode>;

    fn deref(&self) -> &Self::Target {
        &self.root_dir
    }
}

impl Fat32FileSystem {
    pub fn new(device: Arc<DirectoryTreeNode>) -> Result<Self, Error<()>> {
        let disk = Fat32Disk {
            inner: SpinMutex::new((device, 0)),
        };

        let fs = fatfs::FileSystem::new(disk, fatfs::FsOptions::new())?;

        let _holding = Arc::new(fs);
        let pinned = _holding.as_ref()
            as *const fatfs::FileSystem<Fat32Disk, NullTimeProvider, LossyOemCpConverter>;
        let inner = unsafe { pinned.as_ref().unwrap().root_dir() };

        let inode = FatDirectoryInode {
            // Since the "/" is used as separator in the path module and is ignored by the iterator
            // We use "" as the filename for the root directory
            filename: String::from(FILESYSTEM_NAME),
            inner,
            _holding,
        };

        Ok(Fat32FileSystem {
            root_dir: Arc::new(inode),
        })
    }
}

impl fatfs::IoBase for Fat32Disk {
    type Error = ();
}

impl fatfs::Read for Fat32Disk {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ()> {
        let mut inner = self.inner.lock();

        let bytes_read = inner.0.readat(inner.1 as usize, buf).map_err(|_| ())?;

        inner.1 += bytes_read as u64;

        Ok(bytes_read)
    }
}

impl fatfs::Write for Fat32Disk {
    fn write(&mut self, buf: &[u8]) -> Result<usize, ()> {
        let mut inner = self.inner.lock();

        let bytes_written = inner.0.writeat(inner.1 as usize, buf).map_err(|_| ())?;

        inner.1 += bytes_written as u64;

        Ok(bytes_written)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl fatfs::Seek for Fat32Disk {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        let mut inner = self.inner.lock();
        match pos {
            fatfs::SeekFrom::Start(i) => inner.1 = i,
            fatfs::SeekFrom::Current(i) => inner.1 = (inner.1 as i64 + i) as u64,
            fatfs::SeekFrom::End(_) => unreachable!(),
        }

        Ok(inner.1)
    }
}

fn from_fatfs_error<T>(err: fatfs::Error<T>) -> FileSystemError {
    match err {
        Error::Io(_) => FileSystemError::InternalError,
        Error::UnexpectedEof => FileSystemError::UnexpectedEof,
        Error::WriteZero => FileSystemError::WriteZero,
        Error::InvalidInput => FileSystemError::InvalidInput,
        Error::NotFound => FileSystemError::NotFound,
        Error::AlreadyExists => FileSystemError::AlreadyExists,
        Error::DirectoryIsNotEmpty => FileSystemError::DirectoryNotEmpty,
        Error::CorruptedFileSystem => FileSystemError::FileSystemCorrupted,
        Error::NotEnoughSpace => FileSystemError::SpaceNotEnough,
        Error::InvalidFileNameLength => FileSystemError::PathNameLengthExceeded,
        Error::UnsupportedFileNameCharacter => FileSystemError::PathContainsInvalidCharacter,
        _ => FileSystemError::Unknown,
    }
}

pub struct FatFileInodeInner {
    inner: File<'static, Fat32Disk, NullTimeProvider, LossyOemCpConverter>,
    size: usize,
}

#[allow(dead_code)]
pub struct FatFileInode {
    filename: String,
    inner: SpinMutex<FatFileInodeInner>,
}

unsafe impl Sync for FatFileInode {}
unsafe impl Send for FatFileInode {}

impl IInode for FatFileInode {
    fn metadata(&self) -> InodeMetadata {
        InodeMetadata {
            filename: &self.filename,
            entry_type: DirectoryEntryType::File,
            size: unsafe { self.inner.make_guard_unchecked().size },
        }
    }

    fn readat(
        &self,
        offset: usize,
        buffer: &mut [u8],
    ) -> filesystem_abstractions::FileSystemResult<usize> {
        let mut locked_inner = self.inner.lock();
        let file_size = locked_inner.size;

        let mut bytes_read = 0;

        if offset < file_size {
            let pos = SeekFrom::Start(offset as u64);

            locked_inner.inner.seek(pos).map_err(from_fatfs_error)?;

            let rlen = Ord::min(buffer.len(), file_size - offset);

            locked_inner
                .inner
                .read_exact(&mut buffer[..rlen])
                .map_err(from_fatfs_error)?;

            bytes_read = rlen;
        }

        // Add EOF if reached the end
        if buffer.len() > file_size - offset {
            const EOF: u8 = 0;
            buffer[file_size - offset] = EOF;
            bytes_read += 1;
        }

        Ok(bytes_read)
    }

    fn writeat(
        &self,
        offset: usize,
        buffer: &[u8],
    ) -> filesystem_abstractions::FileSystemResult<usize> {
        let mut locked_inner = self.inner.lock();

        let pos = SeekFrom::Start(offset as u64);
        let curr_off = locked_inner.inner.seek(pos).map_err(from_fatfs_error)? as usize;

        if offset != curr_off {
            let buf: [u8; 512] = [0; 512];
            loop {
                let wlen = Ord::min(offset - locked_inner.size, 512);

                if wlen == 0 {
                    break;
                }
                let real_wlen = locked_inner.inner.write(&buf).map_err(from_fatfs_error)?;
                locked_inner.size += real_wlen;
            }
        }

        locked_inner
            .inner
            .write_all(buffer)
            .map_err(from_fatfs_error)?;

        locked_inner.size = Ord::max(locked_inner.size, offset + buffer.len());

        Ok(0)
    }

    fn flush(&self) -> filesystem_abstractions::FileSystemResult<()> {
        self.inner.lock().inner.flush().map_err(from_fatfs_error)
    }

    fn stat(&self, stat: &mut FileStatistics) -> FileSystemResult<()> {
        let size = unsafe { self.inner.make_guard_unchecked().size as u64 };
        stat.inode_id = 1;
        stat.mode = FileStatisticsMode::FILE;
        stat.link_count = 1;
        stat.uid = 0;
        stat.gid = 0;
        stat.size = size;
        stat.block_size = 512;
        stat.block_count = size / 512;
        stat.rdev = 0;
        Ok(())
    }
}

pub struct FatDirectoryInode {
    filename: String,
    inner: Dir<'static, Fat32Disk, NullTimeProvider, LossyOemCpConverter>,
    _holding: Arc<fatfs::FileSystem<Fat32Disk, NullTimeProvider, LossyOemCpConverter>>,
}

unsafe impl Sync for FatDirectoryInode {}
unsafe impl Send for FatDirectoryInode {}

impl IInode for FatDirectoryInode {
    fn metadata(&self) -> InodeMetadata {
        InodeMetadata {
            filename: &self.filename,
            entry_type: DirectoryEntryType::Directory,
            size: 0,
        }
    }

    fn mkdir(&self, name: &str) -> filesystem_abstractions::FileSystemResult<Arc<dyn IInode>> {
        self.inner.create_dir(name).map_err(from_fatfs_error)?;

        let dir = self.inner.open_dir(name).map_err(from_fatfs_error)?;

        Ok(Arc::new(FatDirectoryInode {
            filename: name.to_string(),
            inner: dir,
            _holding: self._holding.clone(),
        }))
    }

    fn lookup(&self, name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        for entry_result in self.inner.iter() {
            match entry_result {
                Ok(entry) => {
                    let filename = entry.file_name();
                    if filename != name {
                        continue;
                    }

                    if entry.is_dir() {
                        let dir = entry.to_dir();
                        return Ok(Arc::new(FatDirectoryInode {
                            filename,
                            inner: dir,
                            _holding: self._holding.clone(),
                        }));
                    } else if entry.is_file() {
                        let file = entry.to_file();
                        return Ok(Arc::new(FatFileInode {
                            filename,
                            inner: SpinMutex::new(FatFileInodeInner {
                                inner: file,
                                size: entry.len() as usize,
                            }),
                        }));
                    }
                }
                Err(err) => {
                    warn!("Error while iterating over directory: {:?}", err);
                    return FileSystemResult::Err(FileSystemError::InternalError);
                }
            }
        }

        FileSystemResult::Err(FileSystemError::NotFound)
    }

    fn touch(&self, name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        let file = self.inner.create_file(name).map_err(from_fatfs_error)?;

        Ok(Arc::new(FatFileInode {
            filename: name.to_string(),
            inner: SpinMutex::new(FatFileInodeInner {
                inner: file,
                size: 0,
            }),
        }))
    }

    fn rmdir(&self, name: &str) -> FileSystemResult<()> {
        // TODO: Check that if the entry is a directory?
        // Or we should remove this API and use remove only?
        self.inner.remove(name).map_err(from_fatfs_error)
    }

    fn remove(&self, name: &str) -> FileSystemResult<()> {
        self.inner.remove(name).map_err(from_fatfs_error)
    }

    fn read_dir(&self) -> FileSystemResult<Vec<filesystem_abstractions::DirectoryEntry>> {
        let mut entries = Vec::new();

        for entry_result in self.inner.iter() {
            match entry_result {
                Ok(entry) => {
                    let short_name = entry.short_file_name_as_bytes();

                    // skip path::CURRENT_DIRECTORY and path::PARENT_DIRECTORY
                    if short_name == b"." || short_name == b".." {
                        continue;
                    }

                    let filename = entry.file_name();

                    let entry_type = if entry.is_dir() {
                        DirectoryEntryType::Directory
                    } else {
                        DirectoryEntryType::File
                    };

                    entries.push(filesystem_abstractions::DirectoryEntry {
                        filename,
                        entry_type,
                    });
                }
                Err(err) => {
                    warn!("Error while iterating over directory: {:?}", err);
                    return FileSystemResult::Err(FileSystemError::InternalError);
                }
            }
        }

        FileSystemResult::Ok(entries)
    }

    fn stat(&self, stat: &mut FileStatistics) -> FileSystemResult<()> {
        stat.inode_id = 1;
        stat.mode = FileStatisticsMode::DIR;
        stat.link_count = 0;
        stat.uid = 0;
        stat.gid = 0;
        stat.size = 0;
        stat.block_size = 512;
        stat.block_count = 0;
        stat.rdev = 0;
        // TODO: implement access time, modify time and create time
        Ok(())
    }
}
