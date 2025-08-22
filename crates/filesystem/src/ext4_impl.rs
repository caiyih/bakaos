use core::alloc::Layout;
use core::ops::Deref;

use ext4_rs::{Ext4, InodeFileType};
use filesystem_abstractions::{
    DirectoryEntryType, DirectoryTreeNode, FileSystemError, IFileSystem, IInode, InodeMetadata,
};

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use timing::TimeSpec;

const ROOT_INODE: u32 = 2; // ext4_rs/src/ext4_defs/consts.rs#L11

struct Ext4Disk {
    inner: Arc<DirectoryTreeNode>,
}

impl ext4_rs::BlockDevice for Ext4Disk {
    fn read_offset(&self, offset: usize) -> Vec<u8> {
        #[inline(always)]
        fn create_aligned_buffer() -> Vec<u8> {
            const LAYOUT: Layout = unsafe {
                core::alloc::Layout::from_size_align_unchecked(
                    ext4_rs::BLOCK_SIZE,
                    align_of::<usize>(),
                )
            };

            let ptr: *mut u8 = loop {
                let allocatted = unsafe { alloc::alloc::alloc(LAYOUT) };

                if !allocatted.is_null() {
                    break allocatted;
                }

                core::hint::spin_loop();
            };

            let slice = unsafe { core::slice::from_raw_parts_mut(ptr, ext4_rs::BLOCK_SIZE) };
            slice.fill(0);

            unsafe {
                Vec::from_raw_parts(slice.as_mut_ptr(), ext4_rs::BLOCK_SIZE, ext4_rs::BLOCK_SIZE)
            }
        }

        let mut buffer = create_aligned_buffer();

        self.inner
            .readat(offset, &mut buffer)
            .expect("Failed to read data from disk");

        buffer
    }

    fn write_offset(&self, offset: usize, data: &[u8]) {
        self.inner
            .writeat(offset, data)
            .expect("Failed to write data to disk");
    }
}

pub struct Ext4FileSystem {
    root_dir: Arc<dyn IInode>,
}

unsafe impl Send for Ext4FileSystem {}
unsafe impl Sync for Ext4FileSystem {}

const FILESYSTEM_NAME: &str = "Ext4FileSystem";

impl Ext4FileSystem {
    pub fn new(device: Arc<DirectoryTreeNode>) -> Result<Ext4FileSystem, &'static str> {
        let inner = Arc::new(Ext4::open(Arc::new(Ext4Disk { inner: device })));

        let p_super_block = &inner.super_block as *const _ as *const u8;
        let magic = unsafe { p_super_block.add(0x38).cast::<u16>().read_volatile() };

        // Ext magic number
        if magic != 0xEF53 {
            // The clippy bawls at me for using Err(()) here
            return Err("Invalid magic number");
        }

        let root_dir = Arc::new(Ext4Inode {
            inode_id: ROOT_INODE,
            fs: inner,
            filename: String::from(FILESYSTEM_NAME),
            file_type: DirectoryEntryType::Directory,
        });

        Ok(Ext4FileSystem { root_dir })
    }
}

impl IFileSystem for Ext4FileSystem {
    fn root_dir(&self) -> Arc<dyn filesystem_abstractions::IInode> {
        self.root_dir.clone()
    }

    fn name(&self) -> &str {
        FILESYSTEM_NAME
    }
}

impl Deref for Ext4FileSystem {
    type Target = Arc<dyn IInode>;

    fn deref(&self) -> &Self::Target {
        &self.root_dir
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

    fn should_be_link(&self) -> Result<(), FileSystemError> {
        if self.file_type != DirectoryEntryType::Symlink {
            return Err(FileSystemError::NotALink);
        }

        Ok(())
    }
}

impl Ext4Inode {
    fn ensure_inode_size(&self, new_size: usize) -> Result<(), FileSystemError> {
        let inode_ref = self.fs.get_inode_ref(self.inode_id);

        if new_size as u64 > inode_ref.inode.size() {
            const BUFFER_SIZE: usize = 512;
            const BUFFER: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];

            let mut curr = inode_ref.inode.size() as usize;

            while curr < new_size {
                let to_write = (new_size - curr).min(BUFFER_SIZE);

                let bytes_written = self
                    .fs
                    .write_at(self.inode_id, new_size, &BUFFER[..to_write])
                    .map_err(|_| FileSystemError::SpaceNotEnough)?;

                curr += bytes_written;
            }
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
    fn metadata(&self) -> InodeMetadata {
        let inode_ref = self.fs.get_inode_ref(self.inode_id);

        InodeMetadata {
            filename: &self.filename,
            entry_type: self.file_type,
            size: inode_ref.inode.size() as usize,
        }
    }

    fn mkdir(&self, name: &str) -> filesystem_abstractions::FileSystemResult<Arc<dyn IInode>> {
        self.should_be_directory()?;
        let mut nameoff = 0;

        let filetype = InodeFileType::S_IFDIR;

        let mut parent = self.inode_id;

        let inode_id = self
            .fs
            .generic_open(name, &mut parent, true, filetype.bits(), &mut nameoff)
            .map_err(|_| FileSystemError::SpaceNotEnough)?;

        Ok(Arc::new(Ext4Inode {
            filename: name.to_string(),
            inode_id,
            file_type: DirectoryEntryType::Directory,
            fs: self.fs.clone(),
        }))
    }

    fn flush(&self) -> filesystem_abstractions::FileSystemResult<()> {
        Ok(())
    }

    fn lookup(&self, name: &str) -> filesystem_abstractions::FileSystemResult<Arc<dyn IInode>> {
        self.should_be_directory()?;

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

    fn read_cache_dir(
        &self,
        caches: &mut BTreeMap<String, Arc<dyn IInode>>,
    ) -> filesystem_abstractions::FileSystemResult<Vec<filesystem_abstractions::DirectoryEntry>>
    {
        #[inline(always)]
        fn to_entry_type(de_type: u8) -> DirectoryEntryType {
            match de_type {
                2 => DirectoryEntryType::Directory,
                _ => DirectoryEntryType::File,
            }
        }

        self.should_be_directory()?;

        let entries = self.fs.dir_get_entries(self.inode_id);

        let mut result = Vec::with_capacity(entries.len());
        for entry in entries {
            if entry.compare_name(path::CURRENT_DIRECTORY)
                || entry.compare_name(path::PARENT_DIRECTORY)
            {
                continue;
            }

            let filename = entry.get_name();
            let file_type = to_entry_type(entry.get_de_type());

            caches.insert(
                filename.clone(),
                Arc::new(Ext4Inode {
                    inode_id: entry.inode,
                    filename: filename.clone(),
                    file_type,
                    fs: self.fs.clone(),
                }),
            );

            result.push(filesystem_abstractions::DirectoryEntry {
                filename,
                entry_type: file_type,
            });
        }

        Ok(result)
    }

    fn touch(&self, name: &str) -> filesystem_abstractions::FileSystemResult<Arc<dyn IInode>> {
        self.should_be_directory()?;
        const MODE: u16 = InodeFileType::S_IFREG.bits() | 0o777;

        let created_inode = self.fs.create(self.inode_id, name, MODE).unwrap();

        Ok(Arc::new(Ext4Inode {
            filename: name.to_string(),
            inode_id: created_inode.inode_num,
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
        self.should_be_directory()?;

        let mut inode_ref = self.fs.get_inode_ref(self.inode_id);
        self.fs
            .dir_remove_entry(&mut inode_ref, name)
            .map_err(|_| FileSystemError::InternalError)?;

        Ok(())
    }

    fn stat(
        &self,
        stat: &mut filesystem_abstractions::FileStatistics,
    ) -> filesystem_abstractions::FileSystemResult<()> {
        let inode_ref = self.fs.get_inode_ref(self.inode_id);

        stat.device_id = 0;
        stat.inode_id = self.inode_id as u64;
        stat.mode = self.file_type.into();
        stat.link_count = inode_ref.inode.links_count() as u32;
        stat.uid = inode_ref.inode.uid() as u32;
        stat.gid = inode_ref.inode.gid() as u32;
        stat.size = inode_ref.inode.size();
        stat.block_size = 4096;
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

        self.ensure_inode_size(offset)?;

        let bytes_written = self
            .fs
            .write_at(self.inode_id, offset, buffer)
            .map_err(|_| FileSystemError::InternalError)?; // TODO: parse the error

        Ok(bytes_written)
    }

    fn hard_link(
        &self,
        name: &str,
        inode: &Arc<dyn IInode>,
    ) -> filesystem_abstractions::FileSystemResult<()> {
        self.should_be_directory()?;

        // Must be an Ext4Inode
        let ext4_inode = inode
            .downcast_ref::<Ext4Inode>()
            .ok_or(FileSystemError::InvalidInput)?;

        if !Arc::ptr_eq(&self.fs, &ext4_inode.fs) {
            return Err(FileSystemError::InvalidInput);
        }

        let mut inode_ref = self.fs.get_inode_ref(self.inode_id);
        let mut child_ref = self.fs.get_inode_ref(ext4_inode.inode_id);
        self.fs
            .link(&mut inode_ref, &mut child_ref, name)
            .map_err(|_| FileSystemError::SpaceNotEnough)?;

        Ok(())
    }

    fn soft_link(
        &self,
        name: &str,
        point_to: &str,
    ) -> filesystem_abstractions::FileSystemResult<Arc<dyn IInode>> {
        self.should_be_directory()?;
        const MODE: u16 = InodeFileType::S_IFLNK.bits() | 0o777;

        let created_inode = self
            .fs
            .create(self.inode_id, name, MODE)
            .map_err(|_| FileSystemError::SpaceNotEnough)?;

        self.fs
            .write_at(created_inode.inode_num, 0, point_to.as_bytes())
            .map_err(|_| FileSystemError::InternalError)?; // TODO: parse the error

        Ok(Arc::new(Ext4Inode {
            filename: name.to_string(),
            inode_id: created_inode.inode_num,
            file_type: DirectoryEntryType::File,
            fs: self.fs.clone(),
        }))
    }

    fn resolve_link(&self) -> Option<String> {
        const BUFFER_LEN: usize = 1024;

        self.should_be_link().ok()?;

        let mut buffer: [u8; BUFFER_LEN] = [0; BUFFER_LEN];

        let bytes_read = self.fs.read_at(self.inode_id, 0, &mut buffer).ok()?;

        assert!(bytes_read <= BUFFER_LEN);

        let target = unsafe { core::str::from_utf8_unchecked(&buffer[..bytes_read]) };

        Some(String::from(target))
    }
}

trait IDirectoryEntryType {
    fn to_entry_type(&self) -> DirectoryEntryType;
}

impl IDirectoryEntryType for ext4_rs::InodeFileType {
    fn to_entry_type(&self) -> DirectoryEntryType {
        if self.contains(InodeFileType::S_IFDIR) {
            DirectoryEntryType::Directory
        } else if self.contains(InodeFileType::S_IFREG) {
            DirectoryEntryType::File
        } else if self.contains(InodeFileType::S_IFLNK) {
            DirectoryEntryType::Symlink
        } else if self.contains(InodeFileType::S_IFBLK) {
            DirectoryEntryType::BlockDevice
        } else if self.contains(InodeFileType::S_IFCHR) {
            DirectoryEntryType::CharDevice
        } else if self.contains(InodeFileType::S_IFIFO) {
            DirectoryEntryType::NamedPipe
        } else {
            log::warn!("Unsupported file type: {self:?}");
            DirectoryEntryType::File
        }
    }
}
