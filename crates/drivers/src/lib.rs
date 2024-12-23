#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use alloc::boxed::Box;

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

pub struct DiskDriver {
    device: Box<dyn IRawDiskDevice>,
}

impl DiskDriver {
    pub fn new(device: Box<dyn IRawDiskDevice>) -> Self {
        DiskDriver { device }
    }

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
                    let (left, right) = buf.split_at_mut(512);
                    let size = self.read_512(left).map_err(|_| bytes_read)?;
                    buf = right;
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

    pub unsafe fn write_at(&mut self, buf: &[u8]) -> Result<usize, usize> {
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
            assert!(buf.len() == 512);
            
            self.device.write_blocks(buf);
            512
        };

        self.device.move_forward(size_written as i64);
        Ok(size_written)
    }

    pub fn get_position(&self) -> usize {
        self.device.get_position()
    }

    pub unsafe fn set_position(&mut self, position: usize) {
        self.device.set_position(position);
    }

    pub unsafe fn move_forward(&mut self, amount: i64) -> usize {
        self.device.move_forward(amount)
    }
}
