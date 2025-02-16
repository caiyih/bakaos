use crate::{
    DirectoryEntryType, FileDescriptorBuilder, FileStatisticsMode, FileSystemResult,
    FrozenFileDescriptorBuilder, ICacheableFile, IInode, InodeMetadata,
};
use alloc::sync::Arc;
use timing::TimeSpec;

use crate::IFile;

pub struct TeleTypewriterBuilder {
    pub stdin_builder: FrozenFileDescriptorBuilder,
    pub stdout_builder: FrozenFileDescriptorBuilder,
    pub stderr_builder: FrozenFileDescriptorBuilder,
}

impl TeleTypewriterBuilder {
    pub fn open_for(task_id: usize) -> Self {
        let tty: Arc<dyn IFile> = Arc::new(TeleTypewriter { task_id });

        let stdin_accessor = tty.cache_as_arc_accessor();
        let stdout_accessor = stdin_accessor.clone_non_inherited_arc();
        let stderr_accessor = stdin_accessor.clone_non_inherited_arc();

        Self {
            stdin_builder: FileDescriptorBuilder::new(stdin_accessor)
                .set_readable()
                .freeze(),
            stdout_builder: FileDescriptorBuilder::new(stdout_accessor)
                .set_writable()
                .freeze(),
            stderr_builder: FileDescriptorBuilder::new(stderr_accessor)
                .set_writable()
                .freeze(),
        }
    }
}

pub trait IStdioFile: IFile {
    fn task_id(&self) -> usize;
}

#[derive(Debug)]
struct TeleTypewriter {
    task_id: usize,
}

impl IStdioFile for TeleTypewriter {
    fn task_id(&self) -> usize {
        self.task_id
    }
}

impl IFile for TeleTypewriter {
    fn metadata(&self) -> Option<Arc<crate::FileMetadata>> {
        None
    }

    fn can_read(&self) -> bool {
        true
    }

    fn read(&self, buf: &mut [u8]) -> usize {
        read(buf)
    }

    fn can_write(&self) -> bool {
        true
    }

    fn write(&self, buf: &[u8]) -> usize {
        write(buf)
    }
}

// TODO: Extract this into separate crate
#[allow(unused_variables)]
fn putchar_to_serial(ch: u8) {
    #[cfg(not(target_arch = "riscv64"))]
    panic!("putchar_to_serial not implemented for this target");

    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") 0x01, // legacy putchar eid
            in("a0") ch,
        );
    }
}

fn getchar_from_serial() -> Option<u8> {
    #[cfg(not(target_arch = "riscv64"))]
    panic!("getchar_from_serial not implemented for this target");

    #[cfg(target_arch = "riscv64")]
    {
        let mut ch: i8;
        unsafe {
            core::arch::asm!(
                "ecall",
                in("a7") 0x02, // legacy getchar eid
                lateout("a0") ch,
            );
        }

        match ch {
            -1 => None,
            _ => Some(ch as u8),
        }
    }
}

fn read(buf: &mut [u8]) -> usize {
    let mut read_bytes = 0;

    while let Some(ch) = getchar_from_serial() {
        buf[read_bytes] = ch;
        read_bytes += 1;

        if read_bytes >= buf.len() {
            break;
        }
    }

    read_bytes
}

fn write(buf: &[u8]) -> usize {
    let mut written_bytes = 0;

    for &ch in buf {
        putchar_to_serial(ch);
        written_bytes += 1;
    }

    written_bytes
}

pub struct TeleTypewriterInode;

impl TeleTypewriterInode {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Arc<dyn IInode> {
        Arc::new(TeleTypewriterInode)
    }
}

impl IInode for TeleTypewriterInode {
    fn metadata(&self) -> InodeMetadata {
        InodeMetadata {
            filename: "tty",
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
        Ok(read(buffer))
    }

    fn writeat(&self, _offset: usize, buffer: &[u8]) -> FileSystemResult<usize> {
        Ok(write(buffer))
    }
}
