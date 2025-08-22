use crate::{
    DirectoryEntryType, FileDescriptorBuilder, FileStatisticsMode, FileSystemResult,
    FrozenFileDescriptorBuilder, ICacheableFile, IInode, InodeMetadata,
};
use alloc::{collections::vec_deque::VecDeque, sync::Arc};
use hermit_sync::SpinMutex;
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

    fn read_avaliable(&self) -> bool {
        let mut lock = TTY_IN_QUEUE.lock();
        while let Some(ch) = getchar_from_serial() {
            lock.push_back(ch);
        }
        !lock.is_empty()
    }
}

static TTY_IN_QUEUE: SpinMutex<VecDeque<u8>> = SpinMutex::new(VecDeque::new());

fn read(buf: &mut [u8]) -> usize {
    let mut lock = TTY_IN_QUEUE.lock();

    let mut read_bytes = 0;

    for ch in buf.iter_mut() {
        match lock.pop_front() {
            Some(read) => {
                *ch = read;
                read_bytes += 1;
            }
            None => break,
        }
    }

    lock.make_contiguous();

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

fn putchar_to_serial(_c: u8) {
    #[cfg_accessible(platform_specific::console_getchar)]
    platform_specific::console_putchar(_c);
}

#[allow(unreachable_code)]
#[inline(always)]
fn getchar_from_serial() -> Option<u8> {
    #[cfg_accessible(platform_specific::console_getchar)]
    return platform_specific::console_getchar();

    None
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
