use core::{cmp::Ordering, mem::MaybeUninit, slice};

use alloc::{sync::Arc, vec::Vec};
use downcast_rs::{impl_downcast, DowncastSync};

use crate::{DirectoryEntry, FileStatistics, FileSystemError, FileSystemResult, InodeMetadata};

pub trait IInode: DowncastSync + Send + Sync {
    fn metadata(&self) -> FileSystemResult<InodeMetadata> {
        Err(FileSystemError::Unimplemented)
    }

    fn readall(&self) -> FileSystemResult<Vec<u8>> {
        self.readrest_at(0)
    }

    fn readrest_at(&self, offset: usize) -> FileSystemResult<Vec<u8>> {
        self.readvec_at(offset, usize::MAX)
    }

    fn readvec_at(&self, offset: usize, max_length: usize) -> FileSystemResult<Vec<u8>> {
        match self.metadata() {
            Ok(metadata) => {
                let len = Ord::min(metadata.size - offset, max_length);
                let mut buf = Vec::<MaybeUninit<u8>>::with_capacity(len);
                unsafe { buf.set_len(len) };

                // Cast &mut [MaybeUninit<u8>] to &mut [u8] to shut up the clippy
                let slice =
                    unsafe { core::mem::transmute::<&mut [MaybeUninit<u8>], &mut [u8]>(&mut buf) };
                self.readat(offset, slice)?;

                // Cast back to Vec<u8>
                Ok(unsafe { core::mem::transmute::<Vec<MaybeUninit<u8>>, Vec<u8>>(buf) })
            }
            // Fall back path to read 512 bytes at a time until EOF
            // Not recommended for potential reallocation overhead
            Err(_) => {
                let mut read_total: usize = 0;
                let mut tmp = [0u8; 512];
                let mut buf: Vec<u8> = Vec::with_capacity(512);

                loop {
                    let read = self.readat(read_total + offset, &mut tmp)?;

                    if read == 0 {
                        break;
                    }

                    if buf.capacity() < read_total + read {
                        buf.reserve(read);

                        debug_assert!(buf.capacity() >= read_total + read);
                    }

                    unsafe {
                        slice::from_raw_parts_mut(buf.as_mut_ptr().add(read_total), read)
                            .copy_from_slice(&tmp[..read]);
                    }

                    read_total += read;

                    unsafe { buf.set_len(read_total) };

                    match (read.cmp(&512), read_total.cmp(&max_length)) {
                        (Ordering::Less, _) | (_, Ordering::Equal) | (_, Ordering::Greater) => {
                            break
                        }
                        _ => (),
                    }
                }

                Ok(buf)
            }
        }
    }

    fn readat(&self, _offset: usize, _buffer: &mut [u8]) -> FileSystemResult<usize> {
        Err(FileSystemError::NotAFile)
    }

    fn writeat(&self, _offset: usize, _buffer: &[u8]) -> FileSystemResult<usize> {
        Err(FileSystemError::NotAFile)
    }

    fn mkdir(&self, _name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        Err(FileSystemError::NotADirectory)
    }

    fn rmdir(&self, _name: &str) -> FileSystemResult<()> {
        Err(FileSystemError::NotADirectory)
    }

    fn remove(&self, _name: &str) -> FileSystemResult<()> {
        Err(FileSystemError::NotADirectory)
    }

    fn touch(&self, _name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        Err(FileSystemError::NotADirectory)
    }

    fn read_dir(&self) -> FileSystemResult<Vec<DirectoryEntry>> {
        Err(FileSystemError::NotADirectory)
    }

    #[deprecated = "This is an internal method, use global_open or DirectoryTreeNode::open"]
    fn lookup(&self, _name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        Err(FileSystemError::NotADirectory)
    }

    fn flush(&self) -> FileSystemResult<()> {
        Err(FileSystemError::Unimplemented)
    }

    fn stat(&self, _stat: &mut FileStatistics) -> FileSystemResult<()> {
        Err(FileSystemError::Unimplemented)
    }
}

impl_downcast!(sync IInode);
