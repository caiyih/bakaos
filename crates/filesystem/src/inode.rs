use alloc::{sync::Arc, vec::Vec};
use filesystem_abstractions::IInode;
use hermit_sync::{RawSpinMutex, SpinMutex};
use lock_api::{MappedMutexGuard, MutexGuard};

static mut INODE_CACHE: SpinMutex<Vec<Option<Arc<dyn IInode>>>> = SpinMutex::new(Vec::new());

pub trait ICacheableInode: IInode {
    fn cache_as_arc_accessor(self: &Arc<Self>) -> Arc<InodeCacheAccessor>;
    fn cache_as_accessor(self: &Arc<Self>) -> InodeCacheAccessor;
}

impl ICacheableInode for dyn IInode {
    fn cache_as_accessor(self: &Arc<Self>) -> InodeCacheAccessor {
        InodeCacheAccessor::new(self.clone())
    }

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
    fn drop(&mut self) {
        unsafe {
            INODE_CACHE.lock()[self.inode_id] = None;
        }
    }
}
