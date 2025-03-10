use core::cell::UnsafeCell;

use alloc::collections::VecDeque;

use alloc::sync::{Arc, Weak};
use hermit_sync::SpinMutex;

use crate::{
    FileCacheAccessor, FileDescriptorBuilder, FrozenFileDescriptorBuilder, ICacheableFile, IFile,
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

    fn can_write(&self) -> bool {
        // broken pipe
        unsafe { self.read_end_weak.get().as_ref().unwrap().strong_count() > 0 }
    }

    fn can_read(&self) -> bool {
        true
    }
}

pub struct PipeBuilder {
    pub read_end_builder: FrozenFileDescriptorBuilder,
    pub write_end_builder: FrozenFileDescriptorBuilder,
}

impl PipeBuilder {
    pub fn open() -> PipeBuilder {
        let pipe_file: Arc<dyn IFile> = Arc::new(Pipe {
            buf_queue: SpinMutex::new(VecDeque::new()),
            write_end_weak: UnsafeCell::new(Weak::new()),
            read_end_weak: UnsafeCell::new(Weak::new()),
        });

        let read_accessor = pipe_file.cache_as_arc_accessor();
        let write_accessor = pipe_file.cache_as_arc_accessor();

        let pipe = pipe_file.downcast_ref::<Pipe>().unwrap();

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

#[cfg(test)]
mod tests {
    use alloc::vec;

    extern crate std;

    use super::PIPE_LIMIT;
    use core::ops::Deref;
    use std::sync::Arc;

    use super::PipeBuilder;

    #[test]
    fn test_read_handle_readable() {
        let pipe_builder = PipeBuilder::open();
        let read_handle = pipe_builder.read_end_builder.fd_inner();

        assert_eq!(read_handle.can_read, true);
    }

    #[test]
    fn test_read_handle_not_writable() {
        let pipe_builder = PipeBuilder::open();
        let read_handle = pipe_builder.read_end_builder.fd_inner();

        assert_eq!(read_handle.can_write, false);
    }

    #[test]
    fn test_write_handle_writable() {
        let pipe_builder = PipeBuilder::open();
        let write_handle = pipe_builder.write_end_builder.fd_inner();

        assert_eq!(write_handle.can_write, true);
    }

    #[test]
    fn test_write_handle_not_readable() {
        let pipe_builder = PipeBuilder::open();
        let write_handle = pipe_builder.write_end_builder.fd_inner();

        assert_eq!(write_handle.can_read, false);
    }

    #[test]
    fn test_different_cache_accessors() {
        let pipe_builder = PipeBuilder::open();

        let read_handle = pipe_builder.read_end_builder.fd_inner();
        let write_handle = pipe_builder.write_end_builder.fd_inner();

        assert!(!Arc::ptr_eq(
            &read_handle.file_handle,
            &write_handle.file_handle
        ));
    }

    #[test]
    fn test_strong_count_different_for_read_and_write() {
        let pipe_builder = PipeBuilder::open();

        let read_handle = pipe_builder.read_end_builder.fd_inner();
        let write_handle = pipe_builder.write_end_builder.fd_inner();
        let write_handle2 = write_handle.clone();

        assert_eq!(Arc::strong_count(&read_handle.file_handle), 1);
        assert_eq!(Arc::strong_count(&write_handle.file_handle), 2);

        drop(write_handle2);

        assert_eq!(Arc::strong_count(&write_handle.file_handle), 1);
    }

    #[test]
    fn test_same_file_for_read_and_write() {
        let pipe_builder = PipeBuilder::open();

        let read_handle = pipe_builder.read_end_builder.fd_inner();
        let write_handle = pipe_builder.write_end_builder.fd_inner();

        // Can not access at the same time
        let read_file_ptr = Arc::as_ptr(read_handle.file_handle.access_ref().deref());
        let write_file_ptr = Arc::as_ptr(write_handle.file_handle.access_ref().deref());

        assert_eq!(read_file_ptr, write_file_ptr);
    }

    #[test]
    fn test_write_functionality() {
        let pipe_builder = PipeBuilder::open();
        let write_handle = pipe_builder.write_end_builder.fd_inner();

        let data_to_write = b"Hello, Pipe!";
        let bytes_written = write_handle.file_handle.access_ref().write(data_to_write);

        assert_eq!(bytes_written, data_to_write.len());
    }

    #[test]
    fn test_read_functionality() {
        let pipe_builder = PipeBuilder::open();

        let read_handle = pipe_builder.read_end_builder.fd_inner();
        let write_handle = pipe_builder.write_end_builder.fd_inner();

        let data_to_write = b"Hello, Pipe!";
        write_handle.file_handle.access_ref().write(data_to_write);

        let mut read_buffer = [0; 100];
        let bytes_read = read_handle.file_handle.access_ref().read(&mut read_buffer);

        assert_eq!(bytes_read, data_to_write.len());
        assert_eq!(&read_buffer[0..bytes_read], data_to_write);
    }

    #[test]
    fn test_broken_pipe_not_writable() {
        let pipe_builder = PipeBuilder::open();
        let write_handle = pipe_builder.write_end_builder.fd_inner();

        drop(pipe_builder.read_end_builder);

        assert!(!write_handle.file_handle.access_ref().can_write());
    }

    #[test]
    fn test_write_before_close() {
        let pipe_builder = PipeBuilder::open();
        let write_handle = pipe_builder.write_end_builder.fd_inner();

        let data_to_write = b"Hello, Pipe!";
        let bytes_written = write_handle.file_handle.access_ref().write(data_to_write);

        assert_eq!(bytes_written, data_to_write.len());
    }

    #[test]
    fn test_read_after_write_close() {
        let pipe_builder = PipeBuilder::open();

        let read_handle = pipe_builder.read_end_builder.fd_inner();
        let write_handle = pipe_builder.write_end_builder.fd_inner();

        let data_to_write = b"Hello, Pipe!";
        write_handle.file_handle.access_ref().write(data_to_write);

        drop(pipe_builder.write_end_builder);

        let mut read_buffer = [0; 100];
        let bytes_read = read_handle.file_handle.access_ref().read(&mut read_buffer);

        assert_eq!(bytes_read, data_to_write.len());
        assert_eq!(&read_buffer[0..bytes_read], data_to_write);
    }

    #[test]
    fn test_read_again_after_write_close() {
        let pipe_builder = PipeBuilder::open();

        let read_handle = pipe_builder.read_end_builder.fd_inner();
        let write_handle = pipe_builder.write_end_builder.fd_inner();

        let data_to_write = b"Hello, Pipe!";
        write_handle.file_handle.access_ref().write(data_to_write);

        drop(pipe_builder.write_end_builder);

        let mut read_buffer = [0; 100];
        read_handle.file_handle.access_ref().read(&mut read_buffer);

        let bytes_read_again = read_handle.file_handle.access_ref().read(&mut read_buffer);
        assert_eq!(bytes_read_again, 0);
    }

    #[test]
    fn test_write_limit() {
        let pipe_builder = PipeBuilder::open();

        let write_handle = pipe_builder.write_end_builder.fd_inner();

        let large_data = vec![0u8; PIPE_LIMIT + 10];
        let bytes_written = write_handle.file_handle.access_ref().write(&large_data);

        assert!(bytes_written <= PIPE_LIMIT);
    }

    #[test]
    fn test_write_unavailable_after_limit() {
        let pipe_builder = PipeBuilder::open();

        let write_handle = pipe_builder.write_end_builder.fd_inner();

        let large_data = vec![0u8; PIPE_LIMIT + 10];
        write_handle.file_handle.access_ref().write(&large_data);

        assert!(!write_handle.file_handle.access_ref().write_avaliable());
    }

    #[test]
    fn test_read_after_write_limit() {
        let pipe_builder = PipeBuilder::open();

        let read_handle = pipe_builder.read_end_builder.fd_inner();
        let write_handle = pipe_builder.write_end_builder.fd_inner();

        let large_data = vec![0u8; PIPE_LIMIT + 10];
        let bytes_written = write_handle.file_handle.access_ref().write(&large_data);

        let mut read_buffer = vec![0u8; PIPE_LIMIT];
        let bytes_read = read_handle.file_handle.access_ref().read(&mut read_buffer);

        assert_eq!(bytes_read, bytes_written);
    }

    #[test]
    fn test_read_data_matches_after_limit() {
        let pipe_builder = PipeBuilder::open();

        let read_handle = pipe_builder.read_end_builder.fd_inner();
        let write_handle = pipe_builder.write_end_builder.fd_inner();

        let large_data = vec![0u8; PIPE_LIMIT + 10];
        let bytes_written = write_handle.file_handle.access_ref().write(&large_data);

        let mut read_buffer = vec![0u8; PIPE_LIMIT];
        let bytes_read = read_handle.file_handle.access_ref().read(&mut read_buffer);

        assert_eq!(bytes_written, bytes_read);
        assert_eq!(&read_buffer[0..bytes_read], &large_data[0..bytes_read]);
    }

    #[test]
    fn test_write_available_after_read() {
        let pipe_builder = PipeBuilder::open();

        let read_handle = pipe_builder.read_end_builder.fd_inner();
        let write_handle = pipe_builder.write_end_builder.fd_inner();

        let large_data = vec![0u8; PIPE_LIMIT + 10];
        write_handle.file_handle.access_ref().write(&large_data);

        let mut read_buffer = vec![0u8; PIPE_LIMIT];
        read_handle.file_handle.access_ref().read(&mut read_buffer);

        assert!(write_handle.file_handle.access_ref().write_avaliable());
    }
}
