use alloc::collections::VecDeque;

use alloc::sync::Arc;
use hermit_sync::SpinMutex;

use crate::{
    FileDescriptorBuilder, FileMetadata, FrozenPermissionFileDescriptorBuilder, ICacheableFile,
    IFile,
};

struct Pipe {
    buf_queue: SpinMutex<VecDeque<u8>>,
}

impl IFile for Pipe {
    fn read_avaliable(&self) -> bool {
        !self.buf_queue.lock().is_empty()
    }

    fn read(&self, buf: &mut [u8]) -> usize {
        if buf.is_empty() {
            return 0;
        }

        let mut queue = self.buf_queue.lock();

        let mut bytes_read = 0;

        while let Some(byte) = queue.pop_front() {
            buf[bytes_read] = byte;
            bytes_read += 1;

            if bytes_read >= buf.len() {
                break;
            }
        }

        bytes_read
    }

    fn write(&self, buf: &[u8]) -> usize {
        let mut bytes_written = 0;
        let mut queue = self.buf_queue.lock();

        for byte in buf {
            queue.push_back(*byte);
            bytes_written += 1;
        }

        bytes_written
    }

    fn metadata(&self) -> Option<Arc<FileMetadata>> {
        None
    }
}

pub struct PipeBuilder {
    pub read_end: FrozenPermissionFileDescriptorBuilder,
    pub write_end: FrozenPermissionFileDescriptorBuilder,
}

impl PipeBuilder {
    pub fn open() -> PipeBuilder {
        let pipe_file: Arc<dyn IFile> = Arc::new(Pipe {
            buf_queue: SpinMutex::new(VecDeque::new()),
        });

        let accessor = pipe_file.cache_as_arc_accessor();

        PipeBuilder {
            read_end: FileDescriptorBuilder::new(accessor.clone())
                .set_readable()
                .freeze(),
            write_end: FileDescriptorBuilder::new(accessor.clone())
                .set_writable()
                .freeze(),
        }
    }
}
