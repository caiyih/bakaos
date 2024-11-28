use alloc::collections::VecDeque;

use alloc::sync::{Arc, Weak};
use hermit_sync::SpinMutex;

use crate::{
    FileCacheAccessor, FileDescriptorBuilder, FileMetadata, FrozenFileDescriptorBuilder,
    ICacheableFile, IFile,
};

struct Pipe {
    buf_queue: SpinMutex<VecDeque<u8>>,
    write_end_weak: SpinMutex<Weak<FileCacheAccessor>>,
}

impl IFile for Pipe {
    fn read_avaliable(&self) -> bool {
        let strong_count = self.write_end_weak.lock().strong_count();

        // When has write end, we should let the read end yield if the buffer is empty
        // But when the write end is closed, either the buffer is empty or not, we should return let the read end read.
        // they will know whether there is data to read by the return value of read()
        strong_count == 0 || !self.buf_queue.lock().is_empty()
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
    pub read_end_builder: FrozenFileDescriptorBuilder,
    pub write_end_builder: FrozenFileDescriptorBuilder,
}

impl PipeBuilder {
    pub fn open() -> PipeBuilder {
        let pipe = Arc::new(Pipe {
            buf_queue: SpinMutex::new(VecDeque::new()),
            write_end_weak: SpinMutex::new(Weak::new()),
        });

        let pipe_file: Arc<dyn IFile> = pipe.clone();

        let read_accessor = pipe_file.cache_as_arc_accessor();
        let write_accessor = read_accessor.clone_non_inherited_arc();

        let read_end_builder = FileDescriptorBuilder::new(read_accessor)
            .set_readable()
            .freeze();

        *pipe.write_end_weak.lock() = Arc::downgrade(&write_accessor);
        let write_end_builder = FileDescriptorBuilder::new(write_accessor)
            .set_writable()
            .freeze();

        PipeBuilder {
            read_end_builder,
            write_end_builder,
        }
    }
}
