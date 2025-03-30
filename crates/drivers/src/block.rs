use alloc::boxed::Box;
use alloc::sync::Arc;
use filesystem_abstractions::{
    DirectoryEntryType, FileStatistics, FileStatisticsMode, FileSystemResult, IInode, InodeMetadata,
};
use hermit_sync::SpinMutex;

pub const SECTOR_SIZE: usize = 512;

pub trait IRawDiskDevice: Sync + Send {
    fn is_read_only(&self) -> bool;

    fn capacity(&self) -> u64;

    fn read_blocks(&mut self, buf: &mut [u8]);

    fn write_blocks(&mut self, buf: &[u8]);

    fn get_position(&self) -> usize;

    fn set_position(&mut self, position: usize);

    fn move_forward(&mut self, amount: i64) -> usize {
        let new_pos = (self.get_position() as i64 + amount) as usize;
        self.set_position(new_pos);
        new_pos
    }
}

struct DiskDriver {
    device: Box<dyn IRawDiskDevice>,
}

impl DiskDriver {
    /// Populate the buffer with the data read from the disk
    /// # Safety
    /// This function is unsafe because you have to lock the disk driver since
    /// set_position call and read_at call are not atomic
    pub unsafe fn read_at(&mut self, mut buf: &mut [u8]) -> Result<usize, usize> {
        #[cfg(debug_assertions)]
        let expected_read = buf.len();

        let mut bytes_read = 0;

        let mut block_buffer = [0u8; SECTOR_SIZE];

        while !buf.is_empty() {
            if bytes_read != 0 {
                block_buffer.fill(0);
            }

            let block_offset = self.device.get_position() % SECTOR_SIZE;

            self.device.read_blocks(&mut block_buffer);

            let effective_len = buf.len().min(SECTOR_SIZE - block_offset);
            buf[..effective_len]
                .copy_from_slice(&block_buffer[block_offset..block_offset + effective_len]);

            bytes_read += effective_len;
            self.device.move_forward(effective_len as i64);

            buf = &mut buf[effective_len..];
        }

        debug_assert!(buf.is_empty());

        #[cfg(debug_assertions)]
        debug_assert_eq!(bytes_read, expected_read);

        Ok(bytes_read)
    }

    /// Write the buffer to the disk
    /// # Safety
    /// This function is unsafe because you have to lock the disk driver since
    /// set_position call and write_at call are not atomic
    pub unsafe fn write_at(&mut self, mut buf: &[u8]) -> Result<usize, usize> {
        #[cfg(debug_assertions)]
        let expected_written = buf.len();

        let mut bytes_written = 0;

        let mut block_buffer = [0u8; SECTOR_SIZE];

        while !buf.is_empty() {
            if bytes_written != 0 {
                block_buffer.fill(0);
            }

            let block_offset = self.device.get_position() % SECTOR_SIZE;

            // if the remaining buffer is not block aligned or can smaller than a block
            // We have to read the whole block before we can write
            let effective_len = if block_offset != 0 || buf.len() < SECTOR_SIZE {
                self.device.read_blocks(&mut block_buffer);
                buf.len().min(SECTOR_SIZE - block_offset)
            } else {
                SECTOR_SIZE
            };

            block_buffer[block_offset..block_offset + effective_len]
                .copy_from_slice(&buf[..effective_len]);

            self.device.write_blocks(&block_buffer);

            bytes_written += effective_len;
            self.device.move_forward(effective_len as i64);

            buf = &buf[effective_len..];
        }

        debug_assert!(buf.is_empty());

        #[cfg(debug_assertions)]
        debug_assert_eq!(bytes_written, expected_written);

        Ok(bytes_written)
    }

    /// Set the position of the disk driver
    /// # Safety
    /// This function is unsafe because you have to lock the disk driver since
    /// set_position call and read/write calls are not atomic
    pub unsafe fn set_position(&mut self, position: usize) {
        self.device.set_position(position);
    }
}

pub struct BlockDeviceInode {
    inner: SpinMutex<DiskDriver>,
}

impl BlockDeviceInode {
    pub fn new(device: Box<dyn IRawDiskDevice>) -> Arc<Self> {
        Arc::new(Self {
            inner: SpinMutex::new(DiskDriver { device }),
        })
    }
}

impl IInode for BlockDeviceInode {
    fn metadata(&self) -> InodeMetadata {
        InodeMetadata {
            filename: "Block device",
            entry_type: DirectoryEntryType::BlockDevice,
            size: 0,
        }
    }

    fn stat(&self, stat: &mut FileStatistics) -> FileSystemResult<()> {
        stat.device_id = 0;
        stat.inode_id = 0;
        stat.mode = FileStatisticsMode::BLOCK;
        stat.link_count = 1;
        stat.uid = 0;
        stat.gid = 0;
        stat.size = 0;
        stat.block_size = 512;
        stat.block_count = 0;
        stat.rdev = 0;

        // stat.ctime = TimeSpec::zero();
        // stat.mtime = TimeSpec::zero();
        // stat.atime = TimeSpec::zero();

        Ok(())
    }

    fn readat(&self, offset: usize, buffer: &mut [u8]) -> FileSystemResult<usize> {
        let mut inner = self.inner.lock();

        unsafe { inner.set_position(offset) };
        unsafe { Ok(inner.read_at(buffer).unwrap()) }
    }

    fn writeat(&self, offset: usize, buffer: &[u8]) -> FileSystemResult<usize> {
        let mut inner = self.inner.lock();

        unsafe { inner.set_position(offset) };
        unsafe { Ok(inner.write_at(buffer).unwrap()) }
    }
}
