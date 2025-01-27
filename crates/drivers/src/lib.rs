#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use alloc::sync::Arc;

use alloc::boxed::Box;
use filesystem_abstractions::{
    DirectoryEntryType, FileStatistics, FileStatisticsMode, FileSystemResult, IInode, InodeMetadata,
};

pub mod vf2;
use hermit_sync::SpinMutex;
pub use vf2::{VisionFive2Disk, VisionFive2SdMMIO};
pub mod virt;
pub use virt::VirtioDiskDriver;

pub trait IRawDiskDevice: Sync + Send {
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
    fn read_512(&mut self, buf: &mut [u8]) -> Result<usize, ()> {
        let len = buf.len();

        debug_assert!(
            len <= 512,
            "buf.len() must be less than or equal to 512, found: {}",
            len
        );

        let sector_offset = self.device.get_position() % 512;

        // Virtio_driver can only read 512 bytes at a time
        let size_read = if sector_offset != 0 || len < 512 {
            let mut tmp = [0u8; 512];
            self.device.read_blocks(&mut tmp);

            let start = sector_offset;
            let end = (sector_offset + len).min(512);

            buf[..end - start].copy_from_slice(&tmp[start..end]);
            end - start
        } else {
            self.device.read_blocks(buf);
            512
        };

        self.device.move_forward(size_read as i64);
        Ok(size_read)
    }

    /// Populate the buffer with the data read from the disk
    /// # Safety
    /// This function is unsafe because you have to lock the disk driver since
    /// set_position call and read_at call are not atomic
    pub unsafe fn read_at(&mut self, mut buf: &mut [u8]) -> Result<usize, usize> {
        let mut bytes_read = 0;

        while !buf.is_empty() {
            match buf.len() {
                0..=512 => {
                    let size = self.read_512(buf).map_err(|_| bytes_read)?;
                    buf = &mut buf[size..];
                    bytes_read += size;
                }
                _ => {
                    let (left, _) = buf.split_at_mut(512);
                    let size = self.read_512(left).map_err(|_| bytes_read)?;
                    buf = &mut buf[size..];
                    bytes_read += size;
                }
            }
        }

        if buf.is_empty() {
            Ok(bytes_read)
        } else {
            Err(bytes_read)
        }
    }

    fn write_512(&mut self, buf: &[u8]) -> Result<usize, ()> {
        let sector_offset = self.device.get_position() % 512;

        let size_written = if sector_offset != 0 || buf.len() < 512 {
            let mut tmp_buf = [0u8; 512];
            self.device.read_blocks(&mut tmp_buf);

            let start = sector_offset;
            let end = (sector_offset + buf.len()).min(512);

            tmp_buf[start..end].copy_from_slice(&buf[..end - start]);
            self.device.write_blocks(&tmp_buf);
            end - start
        } else {
            self.device.write_blocks(buf);
            512
        };

        self.device.move_forward(size_written as i64);
        Ok(size_written)
    }

    /// Write the buffer to the disk
    /// # Safety
    /// This function is unsafe because you have to lock the disk driver since
    /// set_position call and write_at call are not atomic
    pub unsafe fn write_at(&mut self, mut buf: &[u8]) -> Result<usize, usize> {
        let mut bytes_written = 0;

        while !buf.is_empty() {
            match buf.len() {
                0..=512 => {
                    let size = self.write_512(buf).map_err(|_| bytes_written)?;
                    buf = &buf[size..];
                    bytes_written += size;
                }
                _ => {
                    let (left, _) = buf.split_at(512);
                    let size = self.write_512(left).map_err(|_| bytes_written)?;
                    buf = &buf[size..];
                    bytes_written += size;
                }
            }
        }

        if buf.is_empty() {
            Ok(bytes_written)
        } else {
            Err(bytes_written)
        }
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
    #[allow(clippy::new_ret_no_self)]
    pub fn new(device: Box<dyn IRawDiskDevice>) -> Arc<dyn IInode> {
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
