use core::{cmp::Ordering, mem::MaybeUninit, slice};

use alloc::{sync::Arc, vec::Vec};
use downcast_rs::{impl_downcast, DowncastSync};
use hermit_sync::{RawSpinMutex, SpinMutex};
use lock_api::{MappedMutexGuard, MutexGuard};

use crate::{
    DirectoryEntry, FileStatistics, FileSystemError, FileSystemResult, Metadata, OpenFlags,
};

pub trait IInode: DowncastSync + Send + Sync {
    fn metadata(&self) -> FileSystemResult<Metadata> {
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

    fn lookup(&self, _name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        Err(FileSystemError::NotADirectory)
    }

    fn lookup_recursive(&self, _path: &str) -> FileSystemResult<Arc<dyn IInode>> {
        Err(FileSystemError::NotADirectory)
    }

    fn open(&self, _name: &str, _flags: OpenFlags) -> FileSystemResult<Arc<dyn IInode>> {
        Err(FileSystemError::NotADirectory)
    }

    fn flush(&self) -> FileSystemResult<()> {
        Err(FileSystemError::Unimplemented)
    }

    fn mount(&self, _path: &str) -> FileSystemResult<()> {
        Err(FileSystemError::Unimplemented)
    }

    fn umount(&self) -> FileSystemResult<()> {
        Err(FileSystemError::Unimplemented)
    }

    fn stat(&self, _stat: &mut FileStatistics) -> FileSystemResult<()> {
        Err(FileSystemError::Unimplemented)
    }
}

impl_downcast!(sync IInode);

static mut INODE_CACHE: SpinMutex<Vec<Option<Arc<dyn IInode>>>> = SpinMutex::new(Vec::new());

pub trait ICacheableInode: IInode {
    fn cache_as_arc_accessor(self: &Arc<Self>) -> Arc<InodeCacheAccessor>;
    fn cache_as_accessor(self: &Arc<Self>) -> InodeCacheAccessor;
}

impl ICacheableInode for dyn IInode {
    /// Cache the inode in the kernel's inode table and returns an accessor to the inode.
    /// The accessor wraps the index of the inode in the inode table and is the only way to access the inode.
    /// The inode will be removed from the inode table when the accessor is dropped.
    fn cache_as_accessor(self: &Arc<Self>) -> InodeCacheAccessor {
        InodeCacheAccessor::new(self.clone())
    }

    /// Similar to `cache_as_accessor`, but returns an Arc of the accessor.
    /// See `cache_as_accessor` for more information.
    fn cache_as_arc_accessor(self: &Arc<Self>) -> Arc<InodeCacheAccessor> {
        Arc::new(self.cache_as_accessor())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct InodeCacheAccessor {
    inode_id: usize,
}

impl InodeCacheAccessor {
    pub fn new(inode: Arc<dyn IInode>) -> Self {
        let mut caches = unsafe { INODE_CACHE.lock() };

        let inode_id = match caches.iter().enumerate().find(|x| x.1.is_none()) {
            // Reuse the index of the first None element
            Some((index, _)) => {
                caches[index] = Some(inode);
                index
            }
            // Add a new element to the end of the vector
            None => {
                caches.push(Some(inode));
                caches.len() - 1
            }
        };

        InodeCacheAccessor { inode_id }
    }

    // Access the inode from the inode cache table provided by the accessor.
    pub fn access(&self) -> Arc<dyn IInode> {
        let caches = unsafe { INODE_CACHE.lock() };
        let inode = caches[self.inode_id].clone();
        drop(caches); // prevent deadlock
        inode.unwrap() // unwrap is safe because as long as the accessor exists, the inode will exist
                       // Same appies to the unwrap call in the as_mut method
    }

    pub fn inode_id(&self) -> usize {
        self.inode_id
    }

    /// Access the mutable element of the inode cache table.
    /// # Safety
    /// I made this method unsafe because you can change the value of the inode cache table.
    /// This can be a dangerous operation, so you need to be very careful when using this method.
    ///
    /// # Example
    /// ```no_run
    /// use crate::filesystem::ICacheableInode;
    ///
    /// let text_cache = filesystem::root_filesystem()
    ///    .lookup("/text.txt")
    ///    .expect("text.txt not found")
    ///    .cache_as_accessor();
    ///
    /// let mut pInode = unsafe { text_cache.as_mut() };
    ///
    /// // Update the inode cache with another inode
    ///  *pInode = filesystem::root_filesystem()
    ///     .lookup("/new_text.txt")
    ///     .expect("new_text.txt not found");
    ///
    /// // The cache accessor returns the new inode
    /// let new_text = text_cache.access();
    /// ```
    pub unsafe fn as_mut(&self) -> MappedMutexGuard<'static, RawSpinMutex, Arc<dyn IInode>> {
        let caches = INODE_CACHE.lock();
        MutexGuard::map(caches, |caches| caches[self.inode_id].as_mut().unwrap())
    }
}

impl Drop for InodeCacheAccessor {
    // Remove the inode from the inode cache table when the accessor is dropped.
    fn drop(&mut self) {
        unsafe {
            INODE_CACHE.lock()[self.inode_id] = None;
        }
    }
}
