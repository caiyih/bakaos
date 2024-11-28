use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::{ops::Deref, str};

use drivers::IDiskDevice;
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
    device: Box<dyn IDiskDevice>,
}

unsafe impl Send for Fat32Disk {}

unsafe impl Sync for Fat32Disk {}

impl Deref for Fat32Disk {
    type Target = dyn IDiskDevice;

    fn deref(&self) -> &Self::Target {
        self.device.as_ref()
    }
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
    pub fn new(device: Box<dyn IDiskDevice>) -> Result<Self, Error<()>> {
        let disk = Fat32Disk { device };

        let fs = fatfs::FileSystem::new(disk, fatfs::FsOptions::new())?;
        Ok(Fat32FileSystem { inner: fs })
    }
}

impl fatfs::IoBase for Fat32Disk {
    type Error = ();
}

impl Fat32Disk {
    fn read_inner(&mut self, buf: &mut [u8]) -> Result<usize, ()> {
        let len = buf.len();

        assert!(
            len <= 512,
            "buf.len() must be less than or equal to 512, found: {}",
            len
        );

        let device = &mut self.device;
        let device_offset = device.get_position() % 512;

        // Virtio_driver can only read 512 bytes at a time
        let size_read = if device_offset != 0 || len < 512 {
            let mut tmp = [0u8; 512];
            device.read_blocks(&mut tmp);

            let start = device_offset;
            let end = (device_offset + len).min(512);

            buf[..end - start].copy_from_slice(&tmp[start..end]);
            end - start
        } else {
            device.read_blocks(buf);
            512
        };

        device.move_cursor(size_read);
        Ok(size_read)
    }
}

impl fatfs::Read for Fat32Disk {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ()> {
        self.read_exact(buf).map(|_| buf.len()).map_err(|_| ())
    }

    fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<(), Self::Error> {
        while !buf.is_empty() {
            match buf.len() {
                0..=512 => {
                    let size = self.read_inner(buf)?;
                    buf = &mut buf[size..];
                }
                _ => {
                    let (left, right) = buf.split_at_mut(512);
                    self.read_inner(left)?;
                    buf = right;
                }
            }
        }
        if buf.is_empty() {
            Ok(())
        } else {
            warn!("failed to fill whole buffer in read_exact");
            Err(())
        }
    }
}

impl fatfs::Write for Fat32Disk {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let device = &mut self.device;
        let device_offset = device.get_position() % 512;

        let size_written = if device_offset != 0 || buf.len() < 512 {
            let mut tmp_buf = [0u8; 512];
            device.read_blocks(&mut tmp_buf);

            let start = device_offset;
            let end = (device_offset + buf.len()).min(512);

            tmp_buf[start..end].copy_from_slice(&buf[..end - start]);
            device.write_blocks(&tmp_buf);
            end - start
        } else {
            device.write_blocks(buf);
            512
        };

        device.move_cursor(size_written);
        Ok(size_written)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl fatfs::Seek for Fat32Disk {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        // let device = &mut self.device;
        match pos {
            fatfs::SeekFrom::Start(i) => {
                self.device.set_position(i as usize);
                Ok(i)
            }
            fatfs::SeekFrom::Current(i) => {
                let new_pos = (self.device.get_position() as i64) + i;
                self.device.set_position(new_pos as usize);
                Ok(new_pos as u64)
            }
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
        if offset >= locked_inner.size {
            return Ok(0);
        }

        let pos = SeekFrom::Start(offset as u64);

        locked_inner.inner.seek(pos).map_err(from_fatfs_error)?;

        let len = locked_inner.size as u64;

        let rlen = Ord::min(buffer.len(), len as usize - offset);

        locked_inner
            .inner
            .read_exact(&mut buffer[..rlen])
            .map_err(from_fatfs_error)?;

        Ok(rlen)
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
        stat.link_count = 0;
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

    fn lookup_recursive(&self, path: &str) -> FileSystemResult<Arc<dyn IInode>> {
        let mut subs = path
            .trim_end_matches(path::SEPARATOR) // Remove trailing separator, if any
            .split(path::SEPARATOR);

        match subs.next() {
            Some(curr) => {
                let inode = self.lookup(curr)?;
                match subs.clone().next() {
                    Some(next) => {
                        let next_idx = next.as_ptr() as usize - path.as_ptr() as usize;
                        inode.lookup_recursive(&path[next_idx..])
                    }
                    None => Ok(inode),
                }
            }
            None => Err(FileSystemError::NotFound),
        }
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
