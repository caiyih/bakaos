use alloc::sync::Arc;
use filesystem_abstractions::{FileMetadata, IFile};
use kernel_abstractions::IKernelSerial;

pub struct TeletypewriterFile {
    serial: Arc<dyn IKernelSerial>,
}

impl TeletypewriterFile {
    pub fn new(serial: Arc<dyn IKernelSerial>) -> Arc<Self> {
        Arc::new(Self { serial })
    }
}

unsafe impl Send for TeletypewriterFile {}
unsafe impl Sync for TeletypewriterFile {}

impl IFile for TeletypewriterFile {
    fn metadata(&self) -> Option<Arc<FileMetadata>> {
        None
    }

    fn can_read(&self) -> bool {
        true
    }

    fn can_write(&self) -> bool {
        true
    }

    fn read_avaliable(&self) -> bool {
        true
    }

    fn write_avaliable(&self) -> bool {
        true
    }

    fn write(&self, buf: &[u8]) -> usize {
        let mut bytes_sent = 0;

        for c in buf.iter() {
            if self.serial.send(*c).is_err() {
                break;
            }

            bytes_sent += 1;
        }

        bytes_sent
    }

    fn read(&self, buf: &mut [u8]) -> usize {
        let mut bytes_read = 0;

        for c in buf.iter_mut() {
            match self.serial.recv() {
                None => break,
                Some(byte) => *c = byte,
            }

            bytes_read += 1;
        }

        bytes_read
    }

    fn pread(&self, buf: &mut [u8], offset: u64) -> usize {
        if offset == 0 {
            self.read(buf)
        } else {
            0
        }
    }

    fn pwrite(&self, buf: &[u8], offset: u64) -> usize {
        if offset == 0 {
            self.write(buf)
        } else {
            0
        }
    }
}
