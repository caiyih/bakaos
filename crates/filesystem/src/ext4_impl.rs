use core::{mem::MaybeUninit, panic};

use drivers::DiskDriver;
use ext4_rs::{Ext4, InodeFileType};
use filesystem_abstractions::{
    DirectoryEntryType, FileStatisticsMode, FileSystemError, IFileSystem, IInode,
};

use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use hermit_sync::SpinMutex;
use timing::TimeSpec;

const ROOT_INODE: u32 = 2; // ext4_rs/src/ext4_defs/consts.rs#L11

struct Ext4Disk {
    driver: SpinMutex<DiskDriver>,
}

impl ext4_rs::BlockDevice for Ext4Disk {
    fn read_offset(&self, offset: usize) -> Vec<u8> {
        let mut device = self.driver.lock();

        unsafe { device.set_position(offset) };

        let mut buffer = Vec::<MaybeUninit<u8>>::with_capacity(ext4_rs::BLOCK_SIZE);
        unsafe { buffer.set_len(ext4_rs::BLOCK_SIZE) };

        let mut buf = unsafe { core::mem::transmute::<Vec<MaybeUninit<u8>>, Vec<u8>>(buffer) };
        unsafe {
            device
                .read_at(&mut buf)
                .expect("Failed to read data from disk")
        };

        buf
    }

    fn write_offset(&self, offset: usize, data: &[u8]) {
        let mut device = self.driver.lock();

        unsafe {
            device.set_position(offset);
            device.write_at(data).expect("Failed to write data to disk");
        };
    }
}

pub struct Ext4FileSystem {
    root_dir: Arc<Ext4Inode>,
}

unsafe impl Send for Ext4FileSystem {}
unsafe impl Sync for Ext4FileSystem {}

impl Ext4FileSystem {
    pub fn new(device: DiskDriver) -> Ext4FileSystem {
        let inner = Arc::new(Ext4::open(Arc::new(Ext4Disk {
            driver: SpinMutex::new(device),
        })));

        let root_dir = Arc::new(Ext4Inode {
            inode_id: ROOT_INODE,
            fs: inner,
            filename: String::from(""),
            file_type: DirectoryEntryType::Directory,
        });

        Ext4FileSystem { root_dir }
    }
}

impl IFileSystem for Ext4FileSystem {
    fn root_dir(&'static self) -> Arc<dyn filesystem_abstractions::IInode> {
        self.root_dir.clone()
    }

    fn name(&self) -> &str {
        "Ext4FileSystem"
    }
}

struct Ext4Inode {
    filename: String,
    inode_id: u32,
    file_type: DirectoryEntryType,
    fs: Arc<Ext4>,
}

impl Ext4Inode {
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
}

impl Clone for Ext4Inode {
    fn clone(&self) -> Self {
        Self {
            filename: self.filename.clone(),
            inode_id: self.inode_id,
            file_type: self.file_type,
            fs: self.fs.clone(),
        }
    }
}

impl IInode for Ext4Inode {
    fn metadata(
        &self,
    ) -> filesystem_abstractions::FileSystemResult<filesystem_abstractions::InodeMetadata> {
        let inode_ref = self.fs.get_inode_ref(self.inode_id);

        let children_count = match self.file_type {
            DirectoryEntryType::Directory => self.fs.dir_get_entries(self.inode_id).len(),
            _ => 0,
        };

        Ok(filesystem_abstractions::InodeMetadata {
            filename: &self.filename,
            entry_type: self.file_type,
            size: inode_ref.inode.size() as usize,
            children_count,
        })
    }

    fn mkdir(&self, name: &str) -> filesystem_abstractions::FileSystemResult<Arc<dyn IInode>> {
        self.should_be_directory()?;

        let inode = self
            .fs
            .alloc_inode(true)
            .map_err(|_| FileSystemError::SpaceNotEnough)?; // TODO: parse the error

        let mut this = self.fs.get_inode_ref(self.inode_id);
        let mut that = self.fs.get_inode_ref(inode);

        that.inode.set_file_type(InodeFileType::S_IFDIR);
        self.fs.write_back_inode(&mut that);

        if that.inode.file_type() != InodeFileType::S_IFDIR {
            return Err(FileSystemError::InternalError);
        }

        self.fs
            .dir_add_entry(&mut this, &that, name)
            .map_err(|_| FileSystemError::InternalError)?; // TODO: parse the error

        Ok(Arc::new(Ext4Inode {
            filename: name.to_string(),
            inode_id: inode,
            file_type: DirectoryEntryType::Directory,
            fs: self.fs.clone(),
        }))
    }

    fn flush(&self) -> filesystem_abstractions::FileSystemResult<()> {
        Ok(())
    }

    fn lookup(&self, name: &str) -> filesystem_abstractions::FileSystemResult<Arc<dyn IInode>> {
        self.should_be_directory()?;

        if name == "." {
            return Ok(Arc::new(self.clone()));
        }

        let inode_id = self
            .fs
            .dir_get_entries(self.inode_id)
            .into_iter() // takes ownership of the vector
            .find(|e| e.compare_name(name));

        match inode_id {
            Some(entry) => {
                let inode_ref = self.fs.get_inode_ref(entry.inode);

                Ok(Arc::new(Ext4Inode {
                    filename: entry.get_name(),
                    inode_id: entry.inode,
                    file_type: inode_ref.inode.file_type().to_entry_type(),
                    fs: self.fs.clone(),
                }))
            }
            None => Err(filesystem_abstractions::FileSystemError::NotFound),
        }
    }

    fn read_dir(
        &self,
    ) -> filesystem_abstractions::FileSystemResult<Vec<filesystem_abstractions::DirectoryEntry>>
    {
        self.should_be_directory()?;

        let entries = self.fs.dir_get_entries(self.inode_id);

        let mut result = Vec::with_capacity(entries.len());
        for entry in entries {
            let inode_ref = self.fs.get_inode_ref(entry.inode);

            result.push(filesystem_abstractions::DirectoryEntry {
                filename: entry.get_name(),
                size: inode_ref.inode.size() as usize,
                inode: None, // TODO: Implement this
                entry_type: inode_ref.inode.file_type().to_entry_type(),
            });
        }

        Ok(result)
    }

    fn touch(&self, name: &str) -> filesystem_abstractions::FileSystemResult<Arc<dyn IInode>> {
        self.should_be_directory()?;

        let inode = self
            .fs
            .alloc_inode(false)
            .map_err(|_| FileSystemError::SpaceNotEnough)?; // TODO: parse the error

        let mut this = self.fs.get_inode_ref(self.inode_id);
        let mut that = self.fs.get_inode_ref(inode);

        that.inode.set_file_type(InodeFileType::S_IFREG);
        self.fs.write_back_inode(&mut that);

        if that.inode.file_type() != InodeFileType::S_IFREG {
            return Err(FileSystemError::InternalError);
        }

        self.fs
            .dir_add_entry(&mut this, &that, name)
            .map_err(|_| FileSystemError::InternalError)?; // TODO: parse the error

        Ok(Arc::new(Ext4Inode {
            filename: name.to_string(),
            inode_id: inode,
            file_type: DirectoryEntryType::File,
            fs: self.fs.clone(),
        }))
    }

    fn rmdir(&self, name: &str) -> filesystem_abstractions::FileSystemResult<()> {
        self.should_be_directory()?;

        self.fs
            .dir_remove(self.inode_id, name)
            .map(|_| ())
            .map_err(|_| filesystem_abstractions::FileSystemError::NotFound)
    }

    fn remove(&self, name: &str) -> filesystem_abstractions::FileSystemResult<()> {
        self.rmdir(name) // ??
    }

    fn stat(
        &self,
        stat: &mut filesystem_abstractions::FileStatistics,
    ) -> filesystem_abstractions::FileSystemResult<()> {
        let inode_ref = self.fs.get_inode_ref(self.inode_id);

        stat.device_id = 0;
        stat.inode_id = self.inode_id as u64;
        stat.mode = match self.file_type {
            DirectoryEntryType::File => FileStatisticsMode::FILE,
            DirectoryEntryType::Directory => FileStatisticsMode::DIR,
        };
        stat.link_count = inode_ref.inode.links_count() as u32;
        stat.uid = inode_ref.inode.uid() as u32;
        stat.gid = inode_ref.inode.gid() as u32;
        stat.size = inode_ref.inode.size();
        stat.block_size = 512; // TODO: Figure out if this is correct
        stat.block_count = inode_ref.inode.blocks_count();
        stat.rdev = 0;

        stat.ctime = TimeSpec {
            tv_sec: inode_ref.inode.ctime() as i64,
            tv_nsec: 0,
        };

        stat.mtime = TimeSpec {
            tv_sec: inode_ref.inode.mtime() as i64,
            tv_nsec: 0,
        };

        stat.atime = TimeSpec {
            tv_sec: inode_ref.inode.atime() as i64,
            tv_nsec: 0,
        };

        Ok(())
    }

    fn readat(
        &self,
        offset: usize,
        buffer: &mut [u8],
    ) -> filesystem_abstractions::FileSystemResult<usize> {
        self.should_be_file()?;

        let bytes_read = self
            .fs
            .read_at(self.inode_id, offset, buffer)
            .map_err(|_| FileSystemError::InternalError)?; // TODO: parse the error

        Ok(bytes_read)
    }

    fn writeat(
        &self,
        offset: usize,
        buffer: &[u8],
    ) -> filesystem_abstractions::FileSystemResult<usize> {
        self.should_be_file()?;

        let bytes_written = self
            .fs
            .write_at(self.inode_id, offset, buffer)
            .map_err(|_| FileSystemError::InternalError)?; // TODO: parse the error

        Ok(bytes_written)
    }
}

trait IDirectoryEntryType {
    fn to_entry_type(&self) -> DirectoryEntryType;
}

impl IDirectoryEntryType for ext4_rs::InodeFileType {
    fn to_entry_type(&self) -> DirectoryEntryType {
        match *self {
            ext4_rs::InodeFileType::S_IFDIR => DirectoryEntryType::Directory,
            ext4_rs::InodeFileType::S_IFREG => DirectoryEntryType::File,
            t => panic!("Unsupported file type: {:?}", t),
        }
    }
}
