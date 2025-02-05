#![cfg(target_arch = "riscv64")]

use core::cell::UnsafeCell;

use alloc::{string::String, sync::Arc, vec::Vec};
use filesystem_abstractions::{
    DirectoryEntry, DirectoryEntryType, DirectoryTreeNode, FileStatisticsMode, FileSystemError,
    FileSystemResult, IFileSystem, IInode, InodeMetadata,
};
use hermit_sync::SpinMutex;
use lwext4_rust::{
    self,
    bindings::{O_CREAT, O_RDONLY, O_RDWR, O_TRUNC, O_WRONLY, SEEK_SET},
    Ext4BlockWrapper, Ext4File, InodeTypes, KernelDevOp,
};
use timing::TimeSpec;

pub struct Lwext4FileSystem {
    root: Arc<Lwext4Inode>,
}

unsafe impl Send for Lwext4FileSystem {}
unsafe impl Sync for Lwext4FileSystem {}

impl Lwext4FileSystem {
    pub fn new(device: Arc<DirectoryTreeNode>) -> Result<Lwext4FileSystem, ()> {
        let inner = Ext4BlockWrapper::<Lwext4Disk>::new(Lwext4Disk::new(device)).map_err(|_| ())?;

        Ok(Lwext4FileSystem {
            root: Arc::new(Lwext4Inode {
                path: String::from("/"),
                _inner: UnsafeCell::new(Ext4File::new("/", InodeTypes::EXT4_DE_DIR)),
                fs: Arc::new(inner),
            }),
        })
    }
}

impl IFileSystem for Lwext4FileSystem {
    fn root_dir(&self) -> Arc<dyn IInode> {
        self.root.clone()
    }

    fn name(&self) -> &str {
        "Lwext4FileSystem"
    }
}

fn to_entry_type(inode_type: InodeTypes) -> DirectoryEntryType {
    match inode_type {
        InodeTypes::EXT4_DE_UNKNOWN => DirectoryEntryType::Unknown,
        InodeTypes::EXT4_DE_REG_FILE => DirectoryEntryType::File,
        InodeTypes::EXT4_DE_DIR => DirectoryEntryType::Directory,
        InodeTypes::EXT4_DE_CHRDEV => DirectoryEntryType::CharDevice,
        InodeTypes::EXT4_DE_BLKDEV => DirectoryEntryType::BlockDevice,
        InodeTypes::EXT4_DE_FIFO => DirectoryEntryType::NamedPipe,
        InodeTypes::EXT4_DE_SYMLINK => DirectoryEntryType::Symlink,
        _ => panic!("Unimplemented for {:?}", inode_type),
    }
}

struct Lwext4Inode {
    path: String,
    _inner: UnsafeCell<Ext4File>,
    fs: Arc<Ext4BlockWrapper<Lwext4Disk>>,
}

impl Drop for Lwext4Inode {
    fn drop(&mut self) {
        let _ = self.inner().file_close();
    }
}

impl Lwext4Inode {
    #[inline]
    pub fn new(&self, path: String, file_type: InodeTypes) -> Lwext4Inode {
        let inner = Ext4File::new(&path, file_type);

        Lwext4Inode {
            path,
            _inner: UnsafeCell::new(inner),
            fs: self.fs.clone(),
        }
    }

    #[inline(always)]
    fn file_open(&self, flags: u32) -> FileSystemResult<()> {
        self.inner()
            .file_open(&self.path, flags)
            .map_err(|_| FileSystemError::NotFound)?;

        Ok(())
    }

    fn inner(&self) -> &mut Ext4File {
        unsafe { self._inner.get().as_mut().unwrap_unchecked() }
    }

    fn get_type(&self) -> DirectoryEntryType {
        to_entry_type(self.inner().get_type())
    }

    fn should_be_directory(&self) -> Result<(), FileSystemError> {
        if self.get_type() != DirectoryEntryType::Directory {
            return Err(FileSystemError::NotADirectory);
        }

        Ok(())
    }

    fn should_be_file(&self) -> Result<(), FileSystemError> {
        if self.get_type() != DirectoryEntryType::File {
            return Err(FileSystemError::NotAFile);
        }

        Ok(())
    }

    fn should_be_link(&self) -> Result<(), FileSystemError> {
        if self.get_type() != DirectoryEntryType::Symlink {
            return Err(FileSystemError::NotALink);
        }

        Ok(())
    }
}

unsafe impl Send for Lwext4Inode {}
unsafe impl Sync for Lwext4Inode {}

impl IInode for Lwext4Inode {
    fn metadata(&self) -> InodeMetadata {
        let _ = self.file_open(O_RDONLY);
        let size = self.inner().file_size() as usize;

        InodeMetadata {
            filename: path::get_filename(&self.path),
            entry_type: self.get_type(),
            size,
        }
    }

    fn stat(&self, stat: &mut filesystem_abstractions::FileStatistics) -> FileSystemResult<()> {
        let _ = self.file_open(O_RDONLY);

        stat.device_id = 1;
        stat.inode_id = 1;
        stat.mode = FileStatisticsMode::from_bits_retain(unsafe {
            self.inner().file_mode_get().unwrap_unchecked()
        });
        stat.link_count = 1;
        stat.uid = 0;
        stat.gid = 0;
        stat.size = self.inner().file_size();
        stat.block_size = 4096;
        stat.block_count = self.inner().file_size() / 4096;
        stat.rdev = 0;

        stat.ctime = TimeSpec::zero();
        stat.mtime = TimeSpec::zero();
        stat.atime = TimeSpec::zero();

        Ok(())
    }

    fn readat(&self, offset: usize, buffer: &mut [u8]) -> FileSystemResult<usize> {
        self.should_be_file()?;

        let _ = self.file_open(O_RDONLY);
        let filesize = self.inner().file_size();

        if offset as u64 >= filesize {
            return Ok(0);
        }

        let _ = self.inner().file_seek(offset as i64, SEEK_SET);
        let bytes = self.inner().file_read(buffer);
        let _ = self.inner().file_close();

        if self.path == "/strings.lua" {
            log::error!("{:?}", &buffer[..bytes.unwrap()]);
            // loop {}
        }

        Ok(bytes.unwrap_or(0))
    }

    fn writeat(&self, offset: usize, buffer: &[u8]) -> FileSystemResult<usize> {
        self.should_be_file()?;

        let _ = self.file_open(O_RDWR);

        self.inner()
            .file_seek(offset as i64, SEEK_SET)
            .map_err(|_| FileSystemError::InternalError)?;

        self.inner()
            .file_write(buffer)
            .map_err(|_| FileSystemError::InternalError)
    }

    fn mkdir(&self, name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        self.should_be_directory()?;

        let path = path::combine(&self.path, name).unwrap();

        self.inner()
            .dir_mk(&path)
            .map_err(|_| FileSystemError::SpaceNotEnough)?;

        Ok(Arc::new(self.new(path, InodeTypes::EXT4_DE_DIR)))
    }

    fn rmdir(&self, name: &str) -> FileSystemResult<()> {
        self.should_be_directory()?;

        let path = path::combine(&self.path, name).unwrap();

        let _ = self.inner().dir_rm(&path);

        Ok(())
    }

    fn remove(&self, name: &str) -> FileSystemResult<()> {
        self.should_be_directory()?;

        let path = path::combine(&self.path, name).unwrap();

        let _ = self.inner().file_remove(&path);

        Ok(())
    }

    fn touch(&self, name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        self.should_be_directory()?;

        let path = path::combine(&self.path, name).unwrap();

        if self
            .inner()
            .check_inode_exist(&path, InodeTypes::EXT4_DE_REG_FILE)
        {
            return Ok(Arc::new(self.new(path, InodeTypes::EXT4_DE_REG_FILE)));
        }

        let _ = self
            .inner()
            .file_open(&path, O_WRONLY | O_CREAT | O_TRUNC)
            .map_err(|_| FileSystemError::SpaceNotEnough);

        let _ = self.inner().file_close();

        Ok(Arc::new(self.new(path, InodeTypes::EXT4_DE_REG_FILE)))
    }

    fn read_cache_dir(
        &self,
        _caches: &mut alloc::collections::btree_map::BTreeMap<String, Arc<dyn IInode>>,
    ) -> FileSystemResult<alloc::vec::Vec<filesystem_abstractions::DirectoryEntry>> {
        self.should_be_directory()?;

        let mut entries = Vec::<DirectoryEntry>::new();

        let (mut names, mut itypes) = self
            .inner()
            .lwext4_dir_entries()
            .map_err(|_| FileSystemError::NotADirectory)?;

        let mut count = 0;

        while !names.is_empty() && !itypes.is_empty() {
            let name = names.pop().unwrap();
            let itype = itypes.pop().unwrap();

            let name = unsafe { core::str::from_utf8_unchecked(&name) };

            if name == path::CURRENT_DIRECTORY || name == path::PARENT_DIRECTORY {
                continue;
            }

            entries.push(DirectoryEntry {
                filename: String::from(name),
                entry_type: to_entry_type(itype),
            });

            count += 1;
            // guadually release memory
            if count % 100 == 0 {
                names.shrink_to_fit();
                itypes.shrink_to_fit();
            }
        }

        Ok(entries)
    }

    fn lookup(&self, name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        self.should_be_directory()?;

        let path = path::combine(&self.path, name).unwrap();

        if self
            .inner()
            .check_inode_exist(&path, InodeTypes::EXT4_DE_DIR)
        {
            Ok(Arc::new(self.new(path, InodeTypes::EXT4_DE_DIR)))
        } else if self
            .inner()
            .check_inode_exist(&path, InodeTypes::EXT4_DE_REG_FILE)
        {
            Ok(Arc::new(self.new(path, InodeTypes::EXT4_DE_REG_FILE)))
        } else if self
            .inner()
            .check_inode_exist(&path, InodeTypes::EXT4_DE_SYMLINK)
        {
            Ok(Arc::new(self.new(path, InodeTypes::EXT4_DE_SYMLINK)))
        } else {
            Err(FileSystemError::NotFound)
        }
    }

    fn soft_link(&self, name: &str, point_to: &str) -> FileSystemResult<Arc<dyn IInode>> {
        self.should_be_directory()?;

        let path = path::combine(&self.path, name).unwrap();

        if self
            .inner()
            .check_inode_exist(&path, InodeTypes::EXT4_DE_SYMLINK)
        {
            return Err(FileSystemError::AlreadyExists);
        }

        let _ = self
            .inner()
            .file_open(&path, O_WRONLY | O_CREAT | O_TRUNC)
            .map_err(|_| FileSystemError::SpaceNotEnough);

        let _ = self.inner().file_close();

        let link_inode = self.new(path, InodeTypes::EXT4_DE_SYMLINK);

        let _ = link_inode.file_open(O_RDWR);

        link_inode
            .inner()
            .file_seek(0 as i64, SEEK_SET)
            .map_err(|_| FileSystemError::InternalError)?;

        link_inode
            .inner()
            .file_write(point_to.as_bytes())
            .map_err(|_| FileSystemError::InternalError)?;

        Ok(Arc::new(link_inode))
    }

    fn resolve_link(&self) -> Option<String> {
        const BUFFER_LEN: usize = 1024;

        self.should_be_link().ok()?;

        let mut buffer: [u8; BUFFER_LEN] = [0; BUFFER_LEN];

        let _ = self.file_open(O_RDONLY).ok();

        self.inner().file_seek(0 as i64, SEEK_SET).ok()?;

        let len = self.inner().file_read(&mut buffer).ok()?;

        let target = unsafe { core::str::from_utf8_unchecked(&buffer[..len]) };

        Some(String::from(target))
    }
}

struct Lwext4Disk(SpinMutex<(Arc<DirectoryTreeNode>, u64)>);

impl Lwext4Disk {
    pub fn new(block_device: Arc<DirectoryTreeNode>) -> Lwext4Disk {
        Lwext4Disk(SpinMutex::new((block_device, 0)))
    }
}

impl KernelDevOp for Lwext4Disk {
    type DevType = Lwext4Disk;

    fn write(dev: &mut Self::DevType, buf: &[u8]) -> Result<usize, i32> {
        let dev = dev.0.lock();
        let offset = dev.1 as usize;
        dev.0.writeat(offset, buf).unwrap();
        Ok(buf.len())
    }

    fn read(dev: &mut Self::DevType, buf: &mut [u8]) -> Result<usize, i32> {
        let dev = dev.0.lock();
        let offset = dev.1 as usize;
        dev.0.readat(offset, buf).unwrap();
        Ok(buf.len())
    }

    fn seek(dev: &mut Self::DevType, off: i64, whence: i32) -> Result<i64, i32> {
        let mut dev = dev.0.lock();

        let new_pos = match whence as u32 {
            lwext4_rust::bindings::SEEK_SET => off,
            lwext4_rust::bindings::SEEK_CUR => dev.1 as i64 + off,
            lwext4_rust::bindings::SEEK_END => 2 * 1024 * 1024 * 1024,
            _ => unimplemented!(),
        };

        dev.1 = new_pos as u64;

        Ok(new_pos)
    }

    fn flush(_dev: &mut Self::DevType) -> Result<usize, i32>
    where
        Self: Sized,
    {
        Ok(0)
    }
}
