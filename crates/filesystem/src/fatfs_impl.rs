use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::str;
use drivers::DiskDriver;

use fatfs::{Dir, Error, File, LossyOemCpConverter, NullTimeProvider, Read, Seek, SeekFrom, Write};
use filesystem_abstractions::{
    FileStatistics, FileStatisticsMode, FileSystemError, FileSystemResult, IFileSystem, IInode,
};
use hermit_sync::SpinMutex;
use log::warn;

pub struct Fat32FileSystem {
    inner: fatfs::FileSystem<Fat32Disk, NullTimeProvider, LossyOemCpConverter>,
}

unsafe impl Send for Fat32FileSystem {}

unsafe impl Sync for Fat32FileSystem {}

pub struct Fat32Disk {
    driver: SpinMutex<DiskDriver>,
}

impl IFileSystem for Fat32FileSystem {
    fn root_dir(&'static self) -> alloc::sync::Arc<dyn filesystem_abstractions::IInode> {
        Arc::new(FatDirectoryInode {
            // Since the "/" is used as separator in the path module and is ignored by the iterator
            // We use "" as the filename for the root directory
            filename: String::from(""),
            inner: self.inner.root_dir(),
        })
    }

    fn name(&self) -> &str {
        "Fat32FileSystem"
    }
}

impl Fat32FileSystem {
    pub fn new(device: DiskDriver) -> Result<Self, Error<()>> {
        let disk = Fat32Disk {
            driver: SpinMutex::new(device),
        };

        let fs = fatfs::FileSystem::new(disk, fatfs::FsOptions::new())?;
        Ok(Fat32FileSystem { inner: fs })
    }
}

impl fatfs::IoBase for Fat32Disk {
    type Error = ();
}

impl fatfs::Read for Fat32Disk {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ()> {
        unsafe { self.driver.lock().read_at(buf).map_err(|_| ()) }
    }
}

impl fatfs::Write for Fat32Disk {
    fn write(&mut self, buf: &[u8]) -> Result<usize, ()> {
        unsafe { self.driver.lock().write_at(buf).map_err(|_| ()) }
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl fatfs::Seek for Fat32Disk {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        let mut driver = self.driver.lock();
        match pos {
            fatfs::SeekFrom::Start(i) => {
                unsafe { driver.set_position(i as usize) };
                Ok(i)
            }
            fatfs::SeekFrom::Current(i) => Ok(unsafe { driver.move_forward(i) as u64 }),
            fatfs::SeekFrom::End(_) => unreachable!(),
        }
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
    fn metadata(
        &self,
    ) -> filesystem_abstractions::FileSystemResult<filesystem_abstractions::InodeMetadata> {
        Ok(filesystem_abstractions::InodeMetadata {
            filename: &self.filename,
            entry_type: filesystem_abstractions::DirectoryEntryType::File,
            size: unsafe { self.inner.make_guard_unchecked().size },
            children_count: 0,
        })
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
}

unsafe impl Sync for FatDirectoryInode {}
unsafe impl Send for FatDirectoryInode {}

impl IInode for FatDirectoryInode {
    fn metadata(
        &self,
    ) -> filesystem_abstractions::FileSystemResult<filesystem_abstractions::InodeMetadata> {
        Ok(filesystem_abstractions::InodeMetadata {
            filename: &self.filename,
            entry_type: filesystem_abstractions::DirectoryEntryType::Directory,
            size: 0,
            children_count: usize::MAX,
        })
    }

    fn mkdir(&self, name: &str) -> filesystem_abstractions::FileSystemResult<Arc<dyn IInode>> {
        self.inner.create_dir(name).map_err(from_fatfs_error)?;

        let dir = self.inner.open_dir(name).map_err(from_fatfs_error)?;

        Ok(Arc::new(FatDirectoryInode {
            filename: name.to_string(),
            inner: dir,
        }))
    }

    fn lookup(&self, name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        if name.is_empty() || name == "." {
            return Ok(Arc::new(FatDirectoryInode {
                filename: self.filename.clone(),
                inner: self.inner.clone(),
            }));
        }

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
                    let filename = entry.file_name();
                    let size = entry.len();

                    if entry.is_dir() {
                        entries.push(filesystem_abstractions::DirectoryEntry {
                            // Copy the filename so that the filename below can reuse the local variable above
                            filename: String::from(&filename),
                            entry_type: filesystem_abstractions::DirectoryEntryType::Directory,
                            size: size as usize,
                            inode: Some(Arc::new(FatDirectoryInode {
                                filename,
                                inner: entry.to_dir(),
                            })),
                        });
                    } else if entry.is_file() {
                        entries.push(filesystem_abstractions::DirectoryEntry {
                            filename: String::from(&filename),
                            size: size as usize,
                            entry_type: filesystem_abstractions::DirectoryEntryType::File,
                            inode: Some(Arc::new(FatFileInode {
                                filename,
                                inner: SpinMutex::new(FatFileInodeInner {
                                    inner: entry.to_file(),
                                    size: size as usize,
                                }),
                            })),
                        });
                    } else {
                        warn!("Unknown entry type: {} at: {}", filename, self.filename);
                    }
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
