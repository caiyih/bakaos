use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs::{File, Metadata},
    io::{Error, Read, Seek, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use filesystem_abstractions::{
    DirectoryEntry, DirectoryEntryType, DirectoryTreeNode, FileStatisticsMode, FileSystemError,
    FileSystemResult, IInode, InodeMetadata,
};
use hermit_sync::SpinMutex;
use timing::TimeSpec;

pub struct HostFile {
    name: String,
    path: PathBuf,
    inner: SpinMutex<File>,
}

impl HostFile {
    pub fn open(path: &str) -> Arc<DirectoryTreeNode> {
        let inode = Self::try_open(path).unwrap();

        DirectoryTreeNode::from_inode(None, &inode, None)
    }

    fn try_open(path: &str) -> Result<Arc<dyn IInode>, Error> {
        let file = File::open(path)?;

        let name = Path::new(path)
            .canonicalize()
            .unwrap()
            .file_name()
            .unwrap_or(OsStr::new(""))
            .to_string_lossy()
            .to_string();

        Ok(Arc::new(HostFile {
            name,
            path: PathBuf::from(path),
            inner: SpinMutex::new(file),
        }))
    }

    fn meta(&self) -> Metadata {
        self.inner.lock().metadata().unwrap()
    }

    fn ensure_dir(&self) -> FileSystemResult<()> {
        if to_entry_type(&self.meta()) != DirectoryEntryType::Directory {
            return Err(FileSystemError::NotADirectory);
        }

        Ok(())
    }

    fn ensure_file(&self) -> FileSystemResult<()> {
        if to_entry_type(&self.meta()) != DirectoryEntryType::File {
            return Err(FileSystemError::NotAFile);
        }

        Ok(())
    }
}

fn to_entry_type(meta: &Metadata) -> DirectoryEntryType {
    if meta.is_dir() {
        DirectoryEntryType::Directory
    } else if meta.is_file() {
        DirectoryEntryType::File
    } else if meta.is_symlink() {
        DirectoryEntryType::Symlink
    } else {
        unimplemented!("Not implemented for {:?}", meta);
    }
}

impl IInode for HostFile {
    fn metadata(&self) -> InodeMetadata {
        let meta = self.inner.lock().metadata().unwrap();

        InodeMetadata {
            filename: &self.name,
            entry_type: to_entry_type(&meta),
            size: meta.len() as usize,
        }
    }

    fn flush(&self) -> FileSystemResult<()> {
        self.inner
            .lock()
            .flush()
            .map_err(|_| FileSystemError::InternalError)
    }

    fn lookup(&self, name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        self.ensure_dir()?;

        let path = self.path.join(name);

        HostFile::try_open(path.to_str().unwrap()).map_err(|_| FileSystemError::NotFound)
    }

    fn mkdir(&self, name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        self.ensure_dir()?;

        let path = self.path.join(name);
        std::fs::create_dir(path.clone()).map_err(|_| FileSystemError::InternalError)?;

        HostFile::try_open(path.to_str().unwrap()).map_err(|_| FileSystemError::NotFound)
    }

    fn touch(&self, name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        self.ensure_dir()?;

        let path = self.path.join(name);
        std::fs::File::create(path.clone()).map_err(|_| FileSystemError::InternalError)?;

        HostFile::try_open(path.to_str().unwrap()).map_err(|_| FileSystemError::NotFound)
    }

    fn rename(&self, _old_name: &str, new_name: &str) -> FileSystemResult<()> {
        self.ensure_dir()?;

        std::fs::rename(self.path.clone(), self.path.join(new_name))
            .map_err(|_| FileSystemError::InternalError)
    }

    fn rmdir(&self, _name: &str) -> FileSystemResult<()> {
        self.ensure_dir()?;

        std::fs::remove_dir(self.path.clone()).map_err(|_| FileSystemError::InternalError)
    }

    fn remove(&self, _name: &str) -> FileSystemResult<()> {
        self.ensure_dir()?;

        std::fs::remove_file(self.path.clone()).map_err(|_| FileSystemError::InternalError)
    }

    fn read_cache_dir(
        &self,
        _caches: &mut BTreeMap<String, Arc<dyn IInode>>,
    ) -> FileSystemResult<Vec<DirectoryEntry>> {
        let mut entries = Vec::new();

        let read_dir =
            std::fs::read_dir(self.path.clone()).map_err(|_| FileSystemError::InternalError)?;

        for (entry, meta) in read_dir
            .filter_map(|e| e.ok())
            .filter_map(|e| e.metadata().ok().map(|m| (e, m)))
        {
            entries.push(DirectoryEntry {
                filename: entry.file_name().to_string_lossy().to_string(),
                entry_type: to_entry_type(&meta),
            });
        }

        Ok(entries)
    }

    fn writeat(&self, offset: usize, data: &[u8]) -> FileSystemResult<usize> {
        self.ensure_file()?;

        let mut file = self.inner.lock();
        file.seek(std::io::SeekFrom::Start(offset as u64))
            .map_err(|_| FileSystemError::InternalError)?;

        file.write(data).map_err(|_| FileSystemError::InternalError)
    }

    fn readat(&self, offset: usize, buffer: &mut [u8]) -> FileSystemResult<usize> {
        self.ensure_file()?;

        let mut file = self.inner.lock();
        file.seek(std::io::SeekFrom::Start(offset as u64))
            .map_err(|_| FileSystemError::InternalError)?;

        file.read(buffer)
            .map_err(|_| FileSystemError::InternalError)
    }

    fn resize(&self, new_size: u64) -> FileSystemResult<u64> {
        self.ensure_file()?;

        let file = self.inner.lock();
        file.set_len(new_size)
            .map_err(|_| FileSystemError::InternalError)?;

        Ok(new_size)
    }

    fn stat(&self, stat: &mut filesystem_abstractions::FileStatistics) -> FileSystemResult<()> {
        let meta = self.meta();

        stat.size = meta.len();
        stat.mode = match to_entry_type(&meta) {
            DirectoryEntryType::NamedPipe => FileStatisticsMode::FIFO,
            DirectoryEntryType::CharDevice => FileStatisticsMode::CHAR,
            DirectoryEntryType::Directory => FileStatisticsMode::DIR,
            DirectoryEntryType::BlockDevice => FileStatisticsMode::BLOCK,
            DirectoryEntryType::File => FileStatisticsMode::FILE,
            DirectoryEntryType::Symlink => FileStatisticsMode::LINK,
            DirectoryEntryType::Unknown => FileStatisticsMode::NULL,
        };

        stat.ctime = systime_to_timespec(meta.created().unwrap_or(UNIX_EPOCH));
        stat.atime = systime_to_timespec(meta.accessed().unwrap_or(UNIX_EPOCH));
        stat.mtime = systime_to_timespec(meta.modified().unwrap_or(UNIX_EPOCH));

        // These are only supported on UNIX-like systems.
        // We will not handle them to support more platforms.

        // stat.inode_id = meta.ino();
        // stat.device_id = meta.dev();
        // stat.gid = meta.gid();
        // stat.rdev = meta.rdev();
        // stat.link_count = meta.nlink() as u32;

        Ok(())
    }
}

fn systime_to_timespec(time: SystemTime) -> TimeSpec {
    let duration = time.duration_since(UNIX_EPOCH).unwrap();
    TimeSpec {
        tv_sec: duration.as_secs() as i64,
        tv_nsec: duration.subsec_nanos() as i64,
    }
}
