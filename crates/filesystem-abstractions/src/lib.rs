#![cfg_attr(not(feature = "std"), no_std)]

use alloc::{string::String, sync::Arc};
use bitflags::bitflags;
use timing::TimeSpec;

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod caching;
mod file;
mod inode;
mod pipe;
mod stdio;
mod tree;

pub use caching::*;
pub use file::*;
pub use inode::*;
pub use pipe::*;
pub use stdio::*;
pub use tree::{
    global_mount, global_open, global_umount, initialize, DirectoryTreeNode, MountError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSystemError {
    Unimplemented,
    Unknown,
    InternalError,
    UnexpectedEof,
    WriteZero,
    PathNameLengthExceeded,
    PathContainsInvalidCharacter,
    FileSystemCorrupted,
    InvalidInput,
    NotFound,
    AlreadyExists,
    DirectoryNotEmpty,
    SpaceNotEnough,
    NotAFile,
    NotADirectory,
}

pub type FileSystemResult<T> = Result<T, FileSystemError>;

pub trait IFileSystem: Send + Sync {
    fn root_dir(&self) -> Arc<dyn IInode>;
    fn name(&self) -> &str;
    fn flush(&self) -> FileSystemResult<()> {
        Err(FileSystemError::Unimplemented)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DirectoryEntryType {
    File,
    Directory,
}

pub struct DirectoryEntry {
    pub filename: String,
    pub entry_type: DirectoryEntryType,
}

#[derive(Debug, Clone)]
pub struct InodeMetadata<'a> {
    pub filename: &'a str,
    pub entry_type: DirectoryEntryType,
    pub size: usize,
    pub children_count: usize,
}

#[repr(C)]
pub struct FileStatistics {
    pub device_id: u64,
    pub inode_id: u64,
    pub mode: FileStatisticsMode, // device mode
    pub link_count: u32,          // file link count
    pub uid: u32,                 // file uid
    pub gid: u32,                 // file gid
    pub rdev: u64,
    pub __pad: u64,
    pub size: u64,       // file size
    pub block_size: u32, // block size
    pub __pad2: u32,
    pub block_count: u64, // blocks used count
    pub atime: TimeSpec,  // last access time
    pub mtime: TimeSpec,  // last modify time
    pub ctime: TimeSpec,  // create time
}

bitflags! {
    #[derive(Debug)]
    pub struct FileStatisticsMode: u32 {
        const NULL  = 0;
        /// Type
        const TYPE_MASK = 0o170000;
        /// FIFO
        const FIFO  = 0o010000;
        /// character device
        const CHAR  = 0o020000;
        /// directory
        const DIR   = 0o040000;
        /// block device
        const BLOCK = 0o060000;
        /// ordinary regular file
        const FILE  = 0o100000;
        /// symbolic link
        const LINK  = 0o120000;
        /// socket
        const SOCKET = 0o140000;

        /// Set-user-ID on execution.
        const SET_UID = 0o4000;
        /// Set-group-ID on execution.
        const SET_GID = 0o2000;

        /// Read, write, execute/search by owner.
        const OWNER_MASK = 0o700;
        /// Read permission, owner.
        const OWNER_READ = 0o400;
        /// Write permission, owner.
        const OWNER_WRITE = 0o200;
        /// Execute/search permission, owner.
        const OWNER_EXEC = 0o100;

        /// Read, write, execute/search by group.
        const GROUP_MASK = 0o70;
        /// Read permission, group.
        const GROUP_READ = 0o40;
        /// Write permission, group.
        const GROUP_WRITE = 0o20;
        /// Execute/search permission, group.
        const GROUP_EXEC = 0o10;

        /// Read, write, execute/search by others.
        const OTHER_MASK = 0o7;
        /// Read permission, others.
        const OTHER_READ = 0o4;
        /// Write permission, others.
        const OTHER_WRITE = 0o2;
        /// Execute/search permission, others.
        const OTHER_EXEC = 0o1;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct OpenFlags: usize {
        // reserve 3 bits for the access mode
        const NONE          = 0;
        const O_RDONLY      = 0;
        const O_WRONLY      = 1;
        const O_RDWR        = 2;
        const O_ACCMODE     = 3;
        const O_CREAT       = 0o100;
        const O_EXCL        = 0o200;
        const O_NOCTTY      = 0o400;
        const O_TRUNC       = 0o1000;
        const O_APPEND      = 0o2000;
        const O_NONBLOCK    = 0o4000;
        const O_DSYNC       = 0o10000;
        const O_SYNC        = 0o4010000;
        const O_RSYNC       = 0o4010000;
        const O_DIRECTORY   = 0o200000;
        const O_NOFOLLOW    = 0o400000;
        const O_CLOEXEC     = 0o2000000;

        const O_ASYNC       = 0o20000;
        const O_DIRECT      = 0o40000;
        const O_LARGEFILE   = 0o100000;
        const O_NOATIME     = 0o1000000;
        const O_PATH        = 0o10000000;
        const O_TMPFILE     = 0o20200000;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct FileMode: u32 {
        const S_ISUID  = 0o4000;
        const S_ISGID  = 0o2000;
        const S_ISVTX  = 0o1000;

        const S_IRWXU  = 0o700;
        const S_IRUSR  = 0o400;
        const S_IWUSR  = 0o200;
        const S_IXUSR  = 0o100;

        const S_IRWXG  = 0o070;
        const S_IRGRP  = 0o040;
        const S_IWGRP  = 0o020;
        const S_IXGRP  = 0o010;

        const S_IRWXO  = 0o007;
        const S_IROTH  = 0o004;
        const S_IWOTH  = 0o002;
        const S_IXOTH  = 0o001;
    }
}
