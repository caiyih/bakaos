use core::cmp;

use alloc::sync::Arc;
use filesystem_abstractions::{
    DirectoryEntryType, FileStatisticsMode, FileSystemResult, IInode, InodeMetadata,
};
use hermit_sync::SpinMutex;
use timing::TimeSpec;

const BUFFER_CAPACITY: usize = 4096;

struct RingBuffer {
    buffer: [u8; BUFFER_CAPACITY],
    head: usize,
    tail: usize,
    len: usize,
}

static DMESG_BUFFER: SpinMutex<RingBuffer> = SpinMutex::new(RingBuffer {
    buffer: [0; BUFFER_CAPACITY],
    head: 0,
    tail: 0,
    len: 0,
});

pub fn read_dmesg(buffer: &mut [u8]) -> usize {
    let dmesg = DMESG_BUFFER.lock();
    let read_len = cmp::min(buffer.len(), dmesg.len);

    for (i, ch) in buffer.iter_mut().enumerate().take(read_len) {
        *ch = dmesg.buffer[(dmesg.head + i) % BUFFER_CAPACITY];
    }

    read_len
}

fn push_message(msg_bytes: &[u8]) -> usize {
    let mut dmesg = DMESG_BUFFER.lock();
    let write_len = msg_bytes.len();

    if write_len > BUFFER_CAPACITY {
        let start = write_len - BUFFER_CAPACITY;
        dmesg.buffer.copy_from_slice(&msg_bytes[start..]);
        dmesg.head = 0;
        dmesg.tail = 0;
        dmesg.len = BUFFER_CAPACITY;
        return BUFFER_CAPACITY;
    }

    while dmesg.len + write_len > BUFFER_CAPACITY {
        dmesg.head = (dmesg.head + 1) % BUFFER_CAPACITY;
        dmesg.len -= 1;
    }

    for &b in msg_bytes {
        let tail = dmesg.tail;
        dmesg.buffer[tail] = b;
        dmesg.tail = (tail + 1) % BUFFER_CAPACITY;
    }

    dmesg.len += write_len;
    debug_assert!(dmesg.len <= BUFFER_CAPACITY);

    write_len
}

pub struct KernelMessageInode;

impl KernelMessageInode {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Arc<dyn IInode> {
        Arc::new(Self)
    }
}

impl IInode for KernelMessageInode {
    fn metadata(&self) -> filesystem_abstractions::InodeMetadata {
        InodeMetadata {
            filename: "kmsg",
            entry_type: DirectoryEntryType::CharDevice,
            size: DMESG_BUFFER.lock().len,
        }
    }

    fn readat(&self, offset: usize, buffer: &mut [u8]) -> FileSystemResult<usize> {
        let dmesg = DMESG_BUFFER.lock();

        if offset >= dmesg.len {
            return Ok(0);
        }

        let readable_len = cmp::min(buffer.len(), dmesg.len - offset);
        for (i, ch) in buffer.iter_mut().enumerate().take(readable_len) {
            *ch = dmesg.buffer[(dmesg.head + i + offset) % BUFFER_CAPACITY];
        }

        Ok(readable_len)
    }

    fn writeat(&self, _offset: usize, buffer: &[u8]) -> FileSystemResult<usize> {
        Ok(push_message(buffer))
    }

    fn stat(&self, stat: &mut filesystem_abstractions::FileStatistics) -> FileSystemResult<()> {
        stat.device_id = 0;
        stat.inode_id = 0;
        stat.mode = FileStatisticsMode::CHAR;
        stat.link_count = 1;
        stat.uid = 0;
        stat.gid = 0;
        stat.size = DMESG_BUFFER.lock().len as u64;
        stat.block_size = 4096;
        stat.block_count = 1;
        stat.rdev = 0;

        stat.ctime = TimeSpec::zero();
        stat.mtime = TimeSpec::zero();
        stat.atime = TimeSpec::zero();

        Ok(())
    }
}
