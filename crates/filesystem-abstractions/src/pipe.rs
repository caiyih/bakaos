use core::cell::UnsafeCell;

use alloc::collections::VecDeque;

use alloc::sync::{Arc, Weak};
use hermit_sync::SpinMutex;

use crate::{
    FileCacheAccessor, FileDescriptorBuilder, FileMetadata, FrozenFileDescriptorBuilder,
    ICacheableFile, IFile,
};

const PIPE_LIMIT: usize = 1024;

struct Pipe {
    buf_queue: SpinMutex<VecDeque<u8>>,
    write_end_weak: UnsafeCell<Weak<FileCacheAccessor>>,
    read_end_weak: UnsafeCell<Weak<FileCacheAccessor>>,
}

unsafe impl Sync for Pipe {}

impl IFile for Pipe {
    fn write_avaliable(&self) -> bool {
        self.buf_queue.lock().len() < PIPE_LIMIT
    }

    fn read_avaliable(&self) -> bool {
        // Strong counts of accessors that are only inherited by write end
        // Indicates whether the write end is still open
        let strong_count = unsafe { self.write_end_weak.get().as_ref().unwrap().strong_count() };

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

        if buf.len() >= (bytes_read + queue.len()) / 3 {
            queue.make_contiguous();
        }

        bytes_read
    }

    fn write(&self, buf: &[u8]) -> usize {
        let mut bytes_written = 0;
        let mut queue = self.buf_queue.lock();

        for byte in buf {
            if queue.len() >= PIPE_LIMIT {
                break;
            }

            queue.push_back(*byte);
            bytes_written += 1;
        }

        bytes_written
    }

    fn metadata(&self) -> Option<Arc<FileMetadata>> {
        None
    }

    fn can_write(&self) -> bool {
        // broken pipe
        unsafe { self.read_end_weak.get().as_ref().unwrap().strong_count() > 0 }
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
            write_end_weak: UnsafeCell::new(Weak::new()),
            read_end_weak: UnsafeCell::new(Weak::new()),
        });

        let pipe_file: Arc<dyn IFile> = pipe.clone();

        let read_accessor = pipe_file.cache_as_arc_accessor();
        let write_accessor = read_accessor.clone_non_inherited_arc();

        *unsafe { pipe.read_end_weak.get().as_mut().unwrap() } = Arc::downgrade(&read_accessor);
        let read_end_builder = FileDescriptorBuilder::new(read_accessor)
            .set_readable()
            .freeze();

        // All write end file descriptors inherit the same accessor, and we use a weak reference to the accessor to trace whether the write end is still open
        // the weak reference only needs to be store once, so a SpinMutex is too heavy for this
        *unsafe { pipe.write_end_weak.get().as_mut().unwrap() } = Arc::downgrade(&write_accessor);
        let write_end_builder = FileDescriptorBuilder::new(write_accessor)
            .set_writable()
            .freeze();

        PipeBuilder {
            read_end_builder,
            write_end_builder,
        }
    }
}
