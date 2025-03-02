use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    string::{String, ToString},
    sync::{Arc, Weak},
    vec::Vec,
};
use constants::SyscallError;
use core::{cell::UnsafeCell, mem::MaybeUninit, ops::DerefMut};
use hermit_sync::{RwSpinLock, SpinMutex};
use timing::TimeSpec;

use crate::{
    special_inode::{RandomInode, UnblockedRandomInode},
    CachelessInodeFile, DirectoryEntry, DirectoryEntryType, FileMetadata, FileStatistics,
    FileStatisticsMode, FileSystemError, FileSystemResult, IFileSystem, IInode, InodeMetadata,
    NullInode, OpenFlags, TeleTypewriterInode, ZeroInode,
};

use allocation::TrackedFrame;

struct RamFileInodeInner {
    frames: Vec<TrackedFrame>,
    size: usize,
    filename: String,
}

struct RamFileInode {
    inner: RwSpinLock<RamFileInodeInner>,
}

impl RamFileInode {
    fn new(filename: &str) -> Self {
        RamFileInode {
            inner: RwSpinLock::new(RamFileInodeInner {
                frames: Vec::new(),
                size: 0,
                filename: filename.to_string(),
            }),
        }
    }
}

impl IInode for RamFileInode {
    fn metadata(&self) -> InodeMetadata {
        let inner = unsafe { self.inner.data_ptr().as_ref().unwrap() };

        InodeMetadata {
            filename: &inner.filename,
            entry_type: DirectoryEntryType::File,
            size: inner.size,
        }
    }

    fn writeat(&self, offset: usize, buffer: &[u8]) -> FileSystemResult<usize> {
        use address::{IConvertablePhysicalAddress, IPageNum};

        let mut inner = self.inner.write();

        let end_size = offset + buffer.len();

        if end_size > inner.size {
            let required_pages = end_size.div_ceil(constants::PAGE_SIZE);
            inner.frames.resize_with(required_pages, || {
                // TODO: do we have to zero the memory?
                allocation::alloc_frame().expect("Out of memory")
            });
            inner.size = end_size;
        }

        let mut current = offset;
        for frame in &inner.frames[offset / 4096..end_size.div_ceil(constants::PAGE_SIZE)] {
            let in_page_start = current % 4096;
            let in_page_len = usize::min(4096, end_size - current);

            let data_ptr = unsafe {
                frame
                    .ppn()
                    .start_addr()
                    .to_high_virtual()
                    .as_mut_ptr::<u8>()
            };
            let data_slice = unsafe {
                core::slice::from_raw_parts_mut(data_ptr.add(in_page_start), in_page_len)
            };

            data_slice.copy_from_slice(&buffer[current - offset..current - offset + in_page_len]);

            current += in_page_len;
        }

        Ok(current - offset)
    }

    fn readat(&self, offset: usize, buffer: &mut [u8]) -> FileSystemResult<usize> {
        use address::{IConvertablePhysicalAddress, IPageNum};

        let inner = self.inner.read();

        if offset >= inner.size {
            return Ok(0);
        }

        let end_size = usize::min(inner.size, offset + buffer.len());

        let mut current = offset;
        while current < end_size {
            let frame = &inner.frames[current / 4096];
            let in_page_start = current % 4096;
            let in_page_len = usize::min(4096, end_size - current);

            let data_ptr = unsafe { frame.ppn().start_addr().to_high_virtual().as_ptr::<u8>() };
            let data_slice =
                unsafe { core::slice::from_raw_parts(data_ptr.add(in_page_start), in_page_len) };

            buffer[current - offset..current - offset + in_page_len].copy_from_slice(data_slice);

            current += in_page_len;
        }

        Ok(current - offset)
    }

    fn stat(&self, stat: &mut FileStatistics) -> FileSystemResult<()> {
        let inner = self.inner.read();

        stat.device_id = 0;
        stat.inode_id = 0;
        stat.mode = crate::FileStatisticsMode::FILE;
        stat.link_count = 1;
        stat.uid = 0;
        stat.gid = 0;
        stat.size = inner.size as u64;
        stat.block_size = 4096; // PAGE_SIZE
        stat.block_count = inner.frames.len() as u64;
        stat.rdev = 0;

        stat.ctime = TimeSpec::zero();
        stat.mtime = TimeSpec::zero();
        stat.atime = TimeSpec::zero();

        Ok(())
    }

    fn resize(&self, new_size: u64) -> FileSystemResult<u64> {
        let mut inner = self.inner.write();

        if new_size != inner.size as u64 {
            let required_pages = ((new_size + 4095) / 4096) as usize;
            inner.frames.resize_with(required_pages, || {
                allocation::alloc_frame().expect("Out of memory")
            });
            inner.size = new_size as usize;
        }

        Ok(new_size)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MountError {
    InvalidInput,
    NotADirectory,
    FileExists,
    FileNotExists,
    AlreadyMounted,
}

impl MountError {
    fn to_filesystem_error(self) -> FileSystemError {
        match self {
            MountError::InvalidInput => FileSystemError::InvalidInput,
            MountError::NotADirectory => FileSystemError::NotADirectory,
            MountError::FileExists => FileSystemError::AlreadyExists,
            MountError::FileNotExists => FileSystemError::NotFound,
            MountError::AlreadyMounted => FileSystemError::InvalidInput,
        }
    }

    pub fn to_syscall_error(self) -> Result<isize, isize> {
        match self {
            MountError::InvalidInput => SyscallError::InvalidArgument,
            MountError::NotADirectory => SyscallError::NoSuchFileOrDirectory,
            MountError::FileExists => SyscallError::FileExists,
            MountError::FileNotExists => SyscallError::NoSuchFileOrDirectory,
            MountError::AlreadyMounted => SyscallError::DeviceOrResourceBusy,
        }
    }
}

#[derive(Clone)]
enum DirectoryTreeNodeMetadata {
    Inode { inode: Arc<dyn IInode> },
    FileSystem { fs: Arc<dyn IFileSystem> },
    Link { target: String },
    Empty,
}

impl DirectoryTreeNodeMetadata {
    fn as_inode(&self) -> Option<Arc<dyn IInode>> {
        match self {
            DirectoryTreeNodeMetadata::Inode { inode } => Some(inode.clone()),
            DirectoryTreeNodeMetadata::FileSystem { fs } => Some(fs.root_dir()),
            DirectoryTreeNodeMetadata::Link { target: _ } => None,
            DirectoryTreeNodeMetadata::Empty => None,
        }
    }
}

struct DirectoryTreeNodeInner {
    meta: DirectoryTreeNodeMetadata,
    name: String,
    mounted: BTreeMap<String, Arc<DirectoryTreeNode>>,
    opened: BTreeMap<String, Weak<DirectoryTreeNode>>,
    children_cache: BTreeMap<String, Arc<dyn IInode>>,
    shadowed: Option<UnsafeCell<Box<DirectoryTreeNodeInner>>>,
}

impl DirectoryTreeNodeInner {
    fn get_mounted(&self, name: &str) -> Option<Arc<DirectoryTreeNode>> {
        self.mounted.get(name).cloned()
    }

    fn is_mounted(&self, name: &str) -> bool {
        self.get_mounted(name).is_some()
    }
}

pub struct DirectoryTreeNode {
    parent: Option<Arc<DirectoryTreeNode>>,
    inner: SpinMutex<DirectoryTreeNodeInner>,
}

unsafe impl Send for DirectoryTreeNode {}
unsafe impl Sync for DirectoryTreeNode {}

impl DirectoryTreeNode {
    // This method takes ownership of `new` as it's undefined behavior after it was used to shadow
    // Returns whether this operation succeedded. If not, `new` is still valid
    pub fn shadow_with(self: &Arc<DirectoryTreeNode>, new: Arc<DirectoryTreeNode>) -> bool {
        if Arc::ptr_eq(self, &new) {
            return false;
        }

        // This is only used for `new` to drop. Still, doesn't involve any allocation.
        let mut new_inner = DirectoryTreeNodeInner {
            meta: DirectoryTreeNodeMetadata::Empty,
            name: String::new(),
            mounted: BTreeMap::new(),
            opened: BTreeMap::new(),
            children_cache: BTreeMap::new(),
            shadowed: None,
        };

        core::mem::swap(&mut new_inner, new.inner.lock().deref_mut());

        let mut node_inner = self.inner.lock();

        let mut previous_inner = core::mem::replace(node_inner.deref_mut(), new_inner);

        // # SAFETY: This assume that the previous name is always correct
        if node_inner.name != previous_inner.name {
            core::mem::swap(&mut node_inner.name, &mut previous_inner.name);
        }

        node_inner.shadowed = Some(UnsafeCell::new(Box::new(previous_inner)));

        true
    }

    pub fn restore_shadow(self: &Arc<DirectoryTreeNode>) -> Option<Arc<DirectoryTreeNode>> {
        let mut inner = self.inner.lock();

        if let Some(ref shadowed) = inner.shadowed {
            // We have to use a temporary value to make borrow checker happy
            let shadowed_inner = unsafe { shadowed.get().as_mut().unwrap() };

            core::mem::swap(inner.deref_mut(), shadowed_inner);

            // # SAFETY: This assume that the previous name is always correct
            if shadowed_inner.name != inner.name {
                core::mem::swap(&mut shadowed_inner.name, &mut inner.name);
            }

            // prevent memory leak
            let previous_inner = shadowed_inner.shadowed.take().unwrap();

            return Some(Arc::new(DirectoryTreeNode {
                parent: self.parent.clone(),
                inner: SpinMutex::new(*previous_inner.into_inner()),
            }));
        }

        None
    }
}

impl DirectoryTreeNode {
    pub fn open_as_file(
        self: Arc<DirectoryTreeNode>,
        flags: OpenFlags,
        offset: usize,
    ) -> Arc<CachelessInodeFile> {
        Arc::new(CachelessInodeFile {
            metadata: Arc::new(FileMetadata::open(self, flags, offset)),
        })
    }

    pub fn from_empty(
        parent: Option<Arc<DirectoryTreeNode>>,
        name: String,
    ) -> Arc<DirectoryTreeNode> {
        Arc::new(DirectoryTreeNode {
            parent,
            inner: SpinMutex::new(DirectoryTreeNodeInner {
                meta: DirectoryTreeNodeMetadata::Empty,
                name,
                mounted: BTreeMap::new(),
                opened: BTreeMap::new(),
                children_cache: BTreeMap::new(),
                shadowed: None,
            }),
        })
    }

    pub fn from_inode(
        parent: Option<Arc<DirectoryTreeNode>>,
        inode: &Arc<dyn IInode>,
        name: Option<&str>,
    ) -> Arc<DirectoryTreeNode> {
        Arc::new(DirectoryTreeNode {
            parent,
            inner: SpinMutex::new(DirectoryTreeNodeInner {
                meta: DirectoryTreeNodeMetadata::Inode {
                    inode: inode.clone(),
                },
                name: name.unwrap_or(inode.metadata().filename).to_string(),
                mounted: BTreeMap::new(),
                opened: BTreeMap::new(),
                children_cache: BTreeMap::new(),
                shadowed: None,
            }),
        })
    }

    pub fn from_filesystem(
        parent: Option<Arc<DirectoryTreeNode>>,
        fs: Arc<dyn IFileSystem>,
        name: Option<&str>,
    ) -> Arc<DirectoryTreeNode> {
        Arc::new(DirectoryTreeNode {
            parent,
            inner: SpinMutex::new(DirectoryTreeNodeInner {
                name: name.unwrap_or(fs.name()).to_string(),
                meta: DirectoryTreeNodeMetadata::FileSystem { fs },
                mounted: BTreeMap::new(),
                opened: BTreeMap::new(),
                children_cache: BTreeMap::new(),
                shadowed: None,
            }),
        })
    }

    pub fn from_symlink(
        parent: Option<Arc<DirectoryTreeNode>>,
        name: &str,
        target: &str,
    ) -> Arc<DirectoryTreeNode> {
        Arc::new(DirectoryTreeNode {
            parent,
            inner: SpinMutex::new(DirectoryTreeNodeInner {
                name: name.to_string(),
                meta: DirectoryTreeNodeMetadata::Link {
                    target: String::from(target),
                },
                mounted: BTreeMap::new(),
                opened: BTreeMap::new(),
                children_cache: BTreeMap::new(),
                shadowed: None,
            }),
        })
    }

    fn mount_internal(
        self: &Arc<DirectoryTreeNode>,
        node: Arc<DirectoryTreeNode>,
        name: &str,
    ) -> Result<Arc<DirectoryTreeNode>, MountError> {
        #[cfg(debug_assertions)]
        {
            fn get_raw_name(node: &Arc<DirectoryTreeNode>) -> &str {
                match unsafe { &node.inner.data_ptr().as_ref().unwrap().meta } {
                    DirectoryTreeNodeMetadata::Inode { inode } => inode.metadata().filename,
                    DirectoryTreeNodeMetadata::FileSystem { fs } => fs.name(),
                    _ => node.metadata().filename,
                }
            }

            if node.parent.as_ref().is_some_and(|p| Arc::ptr_eq(p, self)) {
                log::info!(
                    "Mounting \"{}\" at \"{}\"",
                    get_raw_name(&node),
                    node.fullpath()
                );
            } else {
                log::warn!(
                    "Mounting \"{}\" at \"{}\", but child's parent was: {:?}. Check the callsite.",
                    get_raw_name(&node),
                    path::combine(&self.fullpath(), name),
                    node.parent.as_ref().map(|p| p.fullpath())
                );
            }
        }

        let mut inner = self.inner.lock();

        if let Some(mounted) = inner.mounted.get(name).cloned() {
            drop(inner);

            mounted.shadow_with(node);
            return Ok(mounted);
        }

        inner
            .mounted
            .insert(name.to_string(), node.clone())
            .map_or_else(|| Ok(node.clone()), |_| Err(MountError::FileExists))
    }

    pub fn mount_as(
        self: &Arc<DirectoryTreeNode>,
        node: Arc<DirectoryTreeNode>,
        name: Option<&str>,
    ) -> Result<Arc<DirectoryTreeNode>, MountError> {
        let name = name.unwrap_or(node.name_internal());

        #[cfg(debug_assertions)]
        if node.parent.is_none()
            || !Arc::ptr_eq(self, unsafe { node.parent.as_ref().unwrap_unchecked() })
        {
            log::warn!("Mounting a node that does not belong to current node. Check the callsite or ensure its a hard link operation.");
        }

        self.mount_internal(node, name)
    }

    pub fn mount_empty(
        self: &Arc<DirectoryTreeNode>,
        name: &str,
    ) -> Result<Arc<DirectoryTreeNode>, MountError> {
        let node = Self::from_empty(Some(self.clone()), name.to_string());

        self.mount_internal(node, name)
    }

    pub fn umount_at(&self, name: &str) -> Result<Arc<DirectoryTreeNode>, MountError> {
        let (name_string, umounted) = self
            .inner
            .lock()
            .mounted
            .remove_entry(name)
            .ok_or(MountError::FileNotExists)?;

        if umounted.inner.lock().shadowed.is_some() {
            umounted.restore_shadow();

            self.inner
                .lock()
                .mounted
                .insert(name_string, umounted.clone());

            return Ok(umounted);
        }

        Ok(umounted)
    }

    pub fn name(&self) -> &str {
        self.name_internal()
    }

    fn name_internal(&self) -> &'static str {
        unsafe { &self.inner.data_ptr().as_ref().unwrap().name }
    }

    pub fn close(&self, name: &str) -> (bool, bool) {
        let mut inner = self.inner.lock();

        let closed = inner.opened.remove(name);
        let unmounted = inner.mounted.remove(name);
        inner.children_cache.remove(name);

        drop(inner); // prevent deadlock in recursive close

        (closed.is_some(), unmounted.is_some())
    }

    pub fn open(
        self: &Arc<DirectoryTreeNode>,
        path: &str,
    ) -> FileSystemResult<Arc<DirectoryTreeNode>> {
        global_open_raw(path, Some(self))
    }

    pub fn open_child(
        self: &Arc<DirectoryTreeNode>,
        name: &str,
    ) -> FileSystemResult<Arc<DirectoryTreeNode>> {
        debug_assert!(!name.contains(path::SEPARATOR));

        if name == path::CURRENT_DIRECTORY || name.is_empty() {
            return self.resolve_all_link();
        }

        if name == path::PARENT_DIRECTORY {
            return self.parent.as_ref().map_or_else(
                || Ok(self.clone()),
                |parent: &Arc<DirectoryTreeNode>| Ok(parent.clone()),
            );
        }

        let inode = {
            // prevent dead lock in lookup method
            let mut inner = self.inner.lock();

            // mounted node has higher priority, as it can shadow the opened node
            if !inner.mounted.is_empty() {
                if let Some(mounted) = inner.mounted.get(name).cloned() {
                    return Ok(mounted);
                }
            }

            if !inner.opened.is_empty() {
                if let Some(opened) = inner.opened.get(name).and_then(|weak| weak.upgrade()) {
                    return Ok(opened);
                }
            }

            #[allow(deprecated)]
            match inner.children_cache.remove(name) {
                Some(cached) => cached,
                None => match inner.meta.as_inode() {
                    Some(inode_inner) => inode_inner.lookup(name)?,
                    None => return Err(FileSystemError::NotFound),
                },
            }
        };

        let opened = Self::from_inode(Some(self.clone()), &inode, Some(name));

        self.inner
            .lock()
            .opened
            .insert(name.to_string(), Arc::downgrade(&opened));

        Ok(opened)
    }

    // if the node was opened in the tree, this returns the full path in the filesystem.
    // if not, the root is considered the deepest node without parent
    pub fn fullpath(self: &Arc<DirectoryTreeNode>) -> String {
        let mut stack = Vec::new();

        {
            let mut current = self.clone();

            while let Some(parent) = &current.parent {
                stack.push(current.name_internal());
                current = parent.clone();
            }
        }

        let size = stack.iter().fold(0, |l, s| l + s.len() + 1); // bytes len with separator
        let mut path = String::with_capacity(size);

        // root
        path.push(path::SEPARATOR);

        while let Some(part) = stack.pop() {
            path.push_str(part);

            if !stack.is_empty() {
                path.push(path::SEPARATOR);
            }
        }

        path
    }

    // Calculate the closest node that's ancestor of both the input node
    pub fn get_common_parent(
        lhs: &Arc<DirectoryTreeNode>,
        rhs: &Arc<DirectoryTreeNode>,
    ) -> Arc<DirectoryTreeNode> {
        let mut lhs_ancestors = BTreeSet::new();
        let mut lhs_current = Some(lhs.clone());

        while let Some(node) = lhs_current {
            lhs_ancestors.insert(Arc::as_ptr(&node));
            lhs_current = node.parent.clone();
        }

        let mut rhs_current = Some(rhs.clone());

        while let Some(node) = rhs_current {
            if lhs_ancestors.contains(&Arc::as_ptr(&node)) {
                return node;
            }
            rhs_current = node.parent.clone();
        }

        unreachable!("All nodes should share at least the root node as a common ancestor");
    }

    // Calculate the closest filesystem of this `DirectoryTreeNode`
    // May be the root whose DirectoryTreeNodeMetadata is not an FileSystem
    pub fn get_containing_filesystem(self: &Arc<DirectoryTreeNode>) -> Arc<DirectoryTreeNode> {
        fn as_filesystem(this: &Arc<DirectoryTreeNode>) -> Option<Arc<DirectoryTreeNode>> {
            match unsafe { &this.inner.data_ptr().as_ref().unwrap().meta } {
                DirectoryTreeNodeMetadata::FileSystem { fs: _ } => Some(this.clone()),
                _ => None,
            }
        }

        let mut current = self.clone();

        loop {
            if let Some(ref parent) = current.parent {
                current = parent.clone();
                continue;
            }

            if let Some(fs_node) = as_filesystem(&current) {
                return fs_node;
            }

            #[cfg(debug_assertions)]
            {
                debug_assert!(current.fullpath() == path::ROOT_STR);
                #[cfg(not(test))]
                debug_assert!(Arc::ptr_eq(&current, unsafe {
                    ROOT.lock().assume_init_ref()
                }))
            }

            return current;
        }
    }
}

impl DirectoryTreeNode {
    pub fn readall(self: &Arc<DirectoryTreeNode>) -> FileSystemResult<Vec<u8>> {
        self.readrest_at(0)
    }

    pub fn readrest_at(self: &Arc<DirectoryTreeNode>, offset: usize) -> FileSystemResult<Vec<u8>> {
        self.readvec_at(offset, usize::MAX)
    }

    pub fn readvec_at(
        self: &Arc<DirectoryTreeNode>,
        offset: usize,
        max_length: usize,
    ) -> FileSystemResult<Vec<u8>> {
        let metadata = self.metadata();
        let len = Ord::min(metadata.size - offset, max_length);
        let mut buf = Vec::<MaybeUninit<u8>>::with_capacity(len);
        unsafe { buf.set_len(len) };

        // Cast &mut [MaybeUninit<u8>] to &mut [u8] to shut up the clippy
        let slice = unsafe { core::mem::transmute::<&mut [MaybeUninit<u8>], &mut [u8]>(&mut buf) };
        self.readat(offset, slice)?;

        // Cast back to Vec<u8>
        Ok(unsafe { core::mem::transmute::<Vec<MaybeUninit<u8>>, Vec<u8>>(buf) })
    }
}

impl Drop for DirectoryTreeNode {
    fn drop(&mut self) {
        if let Some(ref parent) = self.parent {
            parent.close(self.name());
        }
    }
}

impl DirectoryTreeNode {
    pub fn metadata<'a>(self: &'a Arc<DirectoryTreeNode>) -> InodeMetadata<'a> {
        let inner = self.inner.lock();
        let filename = self.name();

        match inner.meta.as_inode() {
            Some(inode) => {
                let meta = inode.metadata();

                InodeMetadata {
                    filename,
                    entry_type: meta.entry_type,
                    size: meta.size,
                }
            }
            None => InodeMetadata {
                filename,
                entry_type: DirectoryEntryType::Directory,
                size: 0,
            },
        }
    }

    pub fn readat(
        self: &Arc<DirectoryTreeNode>,
        offset: usize,
        buffer: &mut [u8],
    ) -> FileSystemResult<usize> {
        match self.inner.lock().meta.as_inode() {
            Some(inode) => inode.readat(offset, buffer),
            None => Err(FileSystemError::NotAFile),
        }
    }

    pub fn writeat(&self, offset: usize, buffer: &[u8]) -> FileSystemResult<usize> {
        match self.inner.lock().meta.as_inode() {
            Some(inode) => inode.writeat(offset, buffer),
            None => Err(FileSystemError::NotAFile),
        }
    }

    pub fn mkdir(
        self: &Arc<DirectoryTreeNode>,
        name: &str,
    ) -> FileSystemResult<Arc<DirectoryTreeNode>> {
        let mut inner = self.inner.lock();

        if inner.is_mounted(name) {
            return Err(FileSystemError::AlreadyExists);
        }

        if let Some(inode) = inner.meta.as_inode() {
            match inode.mkdir(name) {
                Ok(made) => {
                    let wrapped = Self::from_inode(Some(self.clone()), &made, None);

                    inner
                        .opened
                        .insert(wrapped.name().to_string(), Arc::downgrade(&wrapped));

                    return Ok(wrapped);
                }
                Err(e) if e == FileSystemError::AlreadyExists => return Err(e),
                _ => (),
            }
        }

        drop(inner); // release lock, as mount operation requires lock

        self.mount_empty(name).map_err(|e| e.to_filesystem_error())
    }

    pub fn rmdir(self: &Arc<DirectoryTreeNode>, name: &str) -> FileSystemResult<()> {
        // FIXME: Do we have to check if it's a directory?
        if self.close(name).1 {
            return Ok(());
        }

        match self.inner.lock().meta.as_inode() {
            Some(inode) => inode.rmdir(name),
            None => Ok(()), // same as below
        }
    }

    pub fn remove(self: &Arc<DirectoryTreeNode>, name: &str) -> FileSystemResult<()> {
        if self.close(name).1 {
            return Ok(());
        }

        match self.inner.lock().meta.as_inode() {
            Some(inode) => inode.remove(name),
            None => Ok(()), // already removed in close method
        }
    }

    pub fn touch(
        self: &Arc<DirectoryTreeNode>,
        name: &str,
    ) -> FileSystemResult<Arc<DirectoryTreeNode>> {
        let mut inner = self.inner.lock();

        if let Some(inode) = inner.meta.as_inode() {
            match inode.touch(name) {
                Ok(touched) => {
                    let wrapped = DirectoryTreeNode::from_inode(Some(self.clone()), &touched, None);

                    inner
                        .opened
                        .insert(wrapped.name().to_string(), Arc::downgrade(&wrapped));

                    return Ok(wrapped);
                }
                Err(e) if e == FileSystemError::AlreadyExists => return Err(e),
                _ => (),
            }
        }

        drop(inner); // release lock, as mount operation requires lock

        let ram_inode: Arc<dyn IInode> = Arc::new(RamFileInode::new(name));

        global_mount_inode(&ram_inode, name, Some(self)).map_err(|e| e.to_filesystem_error())
    }

    pub fn read_dir(self: &Arc<DirectoryTreeNode>) -> FileSystemResult<Vec<DirectoryEntry>> {
        fn to_directory_entry(name: &str, node: &Arc<DirectoryTreeNode>) -> DirectoryEntry {
            let entry_type = match node.inner.lock().meta.as_inode() {
                Some(inode) => inode.metadata().entry_type,
                None => DirectoryEntryType::Directory,
            };

            DirectoryEntry {
                filename: name.to_string(),
                entry_type,
            }
        }

        let mut entries = {
            let mut inner = self.inner.lock();

            match inner.meta.as_inode() {
                Some(inode) => {
                    let mut entries = inode.read_cache_dir(&mut inner.children_cache)?;
                    let mut overrideds = BTreeMap::new();

                    for entry in entries.iter_mut() {
                        if let Some(overrider) = inner.mounted.remove_entry(&entry.filename) {
                            *entry = to_directory_entry(&entry.filename, &overrider.1);
                            overrideds.insert(overrider.0, overrider.1);
                        }
                    }

                    for (name, entry) in inner.mounted.iter() {
                        entries.push(to_directory_entry(name, entry));
                    }

                    for overrider in overrideds {
                        inner.mounted.insert(overrider.0, overrider.1);
                    }

                    entries
                }
                None => inner
                    .mounted
                    .iter()
                    .map(|(name, mounted)| to_directory_entry(name, mounted))
                    .collect(),
            }
        };

        if let Some(ref parent) = self.parent {
            entries.push(to_directory_entry(path::PARENT_DIRECTORY, parent));
        }

        entries.push(to_directory_entry(path::CURRENT_DIRECTORY, self));

        Ok(entries)
    }

    pub fn stat(self: &Arc<DirectoryTreeNode>, stat: &mut FileStatistics) -> FileSystemResult<()> {
        match self.inner.lock().meta.as_inode() {
            Some(inode) => inode.stat(stat),
            None => {
                stat.device_id = 0;
                stat.inode_id = 0;
                stat.mode = match unsafe { &self.inner.data_ptr().as_ref().unwrap().meta } {
                    DirectoryTreeNodeMetadata::Link { target: _ } => FileStatisticsMode::LINK,
                    DirectoryTreeNodeMetadata::Empty => FileStatisticsMode::DIR,
                    _ => unreachable!(),
                };
                stat.link_count = 1;
                stat.uid = 0;
                stat.gid = 0;
                stat.size = 0;
                stat.block_size = 512;
                stat.block_count = 0;
                stat.rdev = 0;

                stat.ctime = TimeSpec::zero();
                stat.mtime = TimeSpec::zero();
                stat.atime = TimeSpec::zero();

                Ok(())
            }
        }
    }

    pub fn hard_link(
        self: &Arc<DirectoryTreeNode>,
        name: &str,
        source: &Arc<DirectoryTreeNode>,
    ) -> FileSystemResult<()> {
        if Arc::ptr_eq(self, source) {
            if let Some(ref inode) = self.inner.lock().meta.as_inode() {
                if let Ok(ret) = inode.hard_link(name, inode) {
                    return Ok(ret);
                }
            }
        } else {
            let under_same_filesystem = Arc::ptr_eq(
                &self.get_containing_filesystem(),
                &source.get_containing_filesystem(),
            );

            if let (true, Some(ref self_inode), Some(ref source_inode)) = (
                under_same_filesystem,
                self.inner.lock().meta.as_inode(),
                source.inner.lock().meta.as_inode(),
            ) {
                if let Ok(ret) = self_inode.hard_link(name, source_inode) {
                    return Ok(ret);
                }
            }
        }

        // // TODO: this is actually copied some metadata
        // // but we have to use this to keep the path correct.
        // let child = if source.parent.is_none()
        //     || !Arc::ptr_eq(self, unsafe { source.parent.as_ref().unwrap_unchecked() })
        // {
        //     Arc::new(DirectoryTreeNode {
        //         parent: Some(self.clone()),
        //         inner: SpinMutex::new(DirectoryTreeNodeInner {
        //             name: String::from(name),
        //             ..source.inner.lock().clone()
        //         }),
        //     })
        // } else {
        //     source.clone()
        // };

        self.mount_as(source.clone(), Some(name))
            .map_err(|e| e.to_filesystem_error())
            .map(|_| ())
    }

    pub fn soft_link(
        self: &Arc<DirectoryTreeNode>,
        name: &str,
        point_to: &str,
    ) -> FileSystemResult<Arc<DirectoryTreeNode>> {
        let self_inner = self.inner.lock();

        match self_inner.meta.as_inode() {
            Some(ref self_inode) => {
                let inode = self_inode.soft_link(name, point_to)?;

                Ok(Self::from_inode(Some(self.clone()), &inode, Some(name)))
            }
            _ => {
                drop(self_inner);

                let node = Self::from_symlink(Some(self.clone()), name, point_to);

                self.mount_as(node, Some(name))
                    .map_err(|e| e.to_filesystem_error())
            }
        }
    }

    pub fn resolve_link(&self) -> Option<String> {
        match &self.inner.lock().meta {
            DirectoryTreeNodeMetadata::Inode { inode } => inode.resolve_link(),
            DirectoryTreeNodeMetadata::Link { target } => Some(target.clone()),
            _ => None,
        }
    }

    pub fn resolve_all_link(
        self: &Arc<DirectoryTreeNode>,
    ) -> FileSystemResult<Arc<DirectoryTreeNode>> {
        const RESOLUTION_LIMIT: usize = 40;

        let mut current = self.clone();
        for _ in 0..RESOLUTION_LIMIT {
            match current.resolve_link() {
                None => return Ok(current),
                Some(target) => match global_open_raw(&target, current.parent.as_ref()) {
                    Ok(node) => current = node,
                    Err(_) => return Err(FileSystemError::NotFound),
                },
            }
        }

        Err(FileSystemError::LinkTooDepth)
    }

    pub fn resize_inode(self: &Arc<DirectoryTreeNode>, new_size: u64) -> FileSystemResult<u64> {
        let inner = self.inner.lock();

        match inner.meta.as_inode() {
            Some(inode) => inode.resize(new_size),
            None => Err(FileSystemError::NotAFile),
        }
    }
}

// The root of the directory tree
static ROOT: SpinMutex<MaybeUninit<Arc<DirectoryTreeNode>>> = SpinMutex::new(MaybeUninit::uninit());

pub fn initialize() {
    let root = DirectoryTreeNode::from_empty(None, String::new());

    for node in [
        "boot", "dev", "etc", "home", "root", "opt", "mnt", "sys", "tmp", "run", "usr", "var",
        "bin", "proc",
    ]
    .iter()
    {
        root.mount_empty(node).unwrap();
    }

    *ROOT.lock() = MaybeUninit::new(root);

    let dev = global_open("/dev", None).unwrap();

    dev.mount_empty("misc").unwrap().touch("rtc").unwrap();

    global_mount_inode(&TeleTypewriterInode::new(), "/dev/tty", None).unwrap();
    global_mount_inode(&NullInode::new(), "/dev/null", None).unwrap();
    global_mount_inode(&ZeroInode::new(), "/dev/zero", None).unwrap();
    global_mount_inode(&RandomInode::new(), "/dev/random", None).unwrap();
    global_mount_inode(&UnblockedRandomInode::new(), "/dev/urandom", None).unwrap();
}

pub fn global_open(
    path: &str,
    relative_to: Option<&Arc<DirectoryTreeNode>>,
) -> FileSystemResult<Arc<DirectoryTreeNode>> {
    global_open_raw(path, relative_to).and_then(|n| n.resolve_all_link())
}

pub fn global_open_raw(
    path: &str,
    relative_to: Option<&Arc<DirectoryTreeNode>>,
) -> FileSystemResult<Arc<DirectoryTreeNode>> {
    let root = match (relative_to, path::is_path_fully_qualified(path)) {
        (_, true) => unsafe { ROOT.lock().assume_init_ref().clone() },
        (Some(root), false) => root.clone(),
        (None, false) => return Err(FileSystemError::InvalidInput),
    };

    let parts = path.split(path::SEPARATOR).skip_while(|d| d.is_empty());

    let mut current = root;
    for part in parts {
        current = current.resolve_all_link()?.open_child(part)?;
    }

    Ok(current)
}

pub fn global_umount(
    path: &str,
    relative_to: Option<&Arc<DirectoryTreeNode>>,
) -> Result<Arc<DirectoryTreeNode>, MountError> {
    let root = match (relative_to, path::is_path_fully_qualified(path)) {
        (_, true) => {
            let root = ROOT.lock();

            let root_node = unsafe { root.assume_init_ref() };

            // Umount root, restoring shadowed node
            if path.trim_start_matches(path::SEPARATOR).is_empty() {
                return root_node.restore_shadow().ok_or(MountError::InvalidInput);
            }

            root_node.clone()
        }
        (Some(root), false) => root.clone(),
        (None, false) => return Err(MountError::InvalidInput),
    };

    let parent_path = path::get_directory_name(path).unwrap_or("");
    let name = path::get_filename(path);

    let parent = global_open(parent_path, Some(&root)).map_err(|_| MountError::FileNotExists)?;

    parent.umount_at(name)
}

pub fn global_mount_filesystem(
    fs: Arc<dyn IFileSystem>,
    path: &str,
    relative_to: Option<&Arc<DirectoryTreeNode>>,
) -> Result<Arc<DirectoryTreeNode>, MountError> {
    global_mount_internal(path, relative_to, |parent, name| {
        DirectoryTreeNode::from_filesystem(parent.cloned(), fs, Some(name))
    })
}

pub fn global_mount(
    node: &Arc<DirectoryTreeNode>,
    path: &str,
    relative_to: Option<&Arc<DirectoryTreeNode>>,
) -> Result<Arc<DirectoryTreeNode>, MountError> {
    global_mount_internal(path, relative_to, |_, _| node.clone())
}

pub fn global_mount_inode(
    inode: &Arc<dyn IInode>,
    path: &str,
    relative_to: Option<&Arc<DirectoryTreeNode>>,
) -> Result<Arc<DirectoryTreeNode>, MountError> {
    global_mount_internal(path, relative_to, |parent, name| {
        DirectoryTreeNode::from_inode(parent.cloned(), inode, Some(name))
    })
}

fn global_mount_internal(
    path: &str,
    relative_to: Option<&Arc<DirectoryTreeNode>>,
    get_node: impl FnOnce(Option<&Arc<DirectoryTreeNode>>, &str) -> Arc<DirectoryTreeNode>,
) -> Result<Arc<DirectoryTreeNode>, MountError> {
    let root = match (relative_to, path::is_path_fully_qualified(path)) {
        (_, true) => {
            let root = ROOT.lock();

            // new root
            if path.trim_start_matches(path::SEPARATOR).is_empty() {
                let root = unsafe { root.assume_init_ref() };
                root.shadow_with(get_node(None, ""));

                return Ok(root.clone());
            }

            unsafe { root.assume_init_ref().clone() }
        }
        (Some(root), false) => root.clone(),
        (None, false) => return Err(MountError::InvalidInput),
    };

    let parent_path = path::get_directory_name(path).unwrap_or("");
    let name = path::get_filename(path);

    let parent = global_open(parent_path, Some(&root)).map_err(|_| MountError::FileNotExists)?;

    let node = get_node(Some(&parent), name);

    parent.mount_as(node, Some(name))
}

pub fn global_find_containing_filesystem(
    path: &str,
    relative_to: Option<&Arc<DirectoryTreeNode>>,
    require_parent_node: bool,
) -> Option<Arc<DirectoryTreeNode>> {
    let search_root = match (relative_to, path::is_path_fully_qualified(path)) {
        (None, false) => return None,
        (_, true) => unsafe { ROOT.lock().assume_init_ref().clone() },
        (Some(relative_to), _) => relative_to.clone(),
    };

    let mut current = search_root.clone();
    let mut deepest_fs = current.get_containing_filesystem();

    let mut path_components = path
        .trim_end_matches(path::SEPARATOR)
        .split(path::SEPARATOR)
        .skip_while(|d| d.is_empty());

    for component in path_components.by_ref() {
        match current.open_child(component) {
            Ok(child) => {
                current = child;
                let fs = current.get_containing_filesystem();
                if !Arc::ptr_eq(&fs, &deepest_fs) {
                    deepest_fs = fs;
                }
            }
            Err(_) => break,
        }
    }

    if require_parent_node {
        path_components.next(); // skip target node

        // parent node does not exist
        if path_components.next().is_some() {
            return None;
        }
    }

    Some(deepest_fs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DirectoryEntryType, FileStatistics, FileStatisticsMode, FileSystemError, IFile,
        IFileSystem, IInode,
    };
    use alloc::{string::ToString, sync::Arc};

    #[test]
    fn test_ram_file_inode_new() {
        let filename = "test_file";
        let inode = RamFileInode::new(filename);
        assert_eq!(inode.metadata().filename, filename);
        assert_eq!(inode.metadata().entry_type, DirectoryEntryType::File);
        assert_eq!(inode.metadata().size, 0);
    }

    #[test]
    fn test_ram_file_inode_stat() {
        let inode = RamFileInode::new("test_file");
        let mut stat = unsafe { core::mem::zeroed::<FileStatistics>() };
        inode.stat(&mut stat).unwrap();

        assert_eq!(stat.mode, FileStatisticsMode::FILE);
        assert_eq!(stat.link_count, 1);
    }

    #[test]
    fn test_ram_file_inode_resize() {
        let inode = RamFileInode::new("test_file");
        let new_size = 0;
        let result = inode.resize(new_size).unwrap();
        assert_eq!(result, new_size);
        assert_eq!(inode.metadata().size, new_size as usize);
    }

    #[test]
    fn test_mount_error_conversion() {
        let errors = [
            MountError::InvalidInput,
            MountError::NotADirectory,
            MountError::FileExists,
            MountError::FileNotExists,
            MountError::AlreadyMounted,
        ];

        for error in errors {
            let fs_error = error.to_filesystem_error();
            let syscall_error = error.to_syscall_error();

            match error {
                MountError::InvalidInput => {
                    assert_eq!(fs_error, FileSystemError::InvalidInput);
                    assert!(syscall_error.is_err());
                }
                MountError::NotADirectory => {
                    assert_eq!(fs_error, FileSystemError::NotADirectory);
                    assert!(syscall_error.is_err());
                }
                MountError::FileExists => {
                    assert_eq!(fs_error, FileSystemError::AlreadyExists);
                    assert!(syscall_error.is_err());
                }
                MountError::FileNotExists => {
                    assert_eq!(fs_error, FileSystemError::NotFound);
                    assert!(syscall_error.is_err());
                }
                MountError::AlreadyMounted => {
                    assert_eq!(fs_error, FileSystemError::InvalidInput);
                    assert!(syscall_error.is_err());
                }
            }
        }
    }

    #[test]
    fn test_directory_tree_node_creation() {
        let parent = None;
        let name = "test_dir";
        let node = DirectoryTreeNode::from_empty(parent, name.to_string());
        assert_eq!(node.name(), name);
        assert_eq!(node.metadata().filename, name);
        assert_eq!(node.metadata().entry_type, DirectoryEntryType::Directory);
        assert_eq!(node.metadata().size, 0);
    }

    #[test]
    fn test_directory_tree_node_mount_and_umount() {
        let parent = DirectoryTreeNode::from_empty(None, "parent".to_string());
        let child = DirectoryTreeNode::from_empty(Some(parent.clone()), "child".to_string());

        parent.mount_as(child.clone(), Some("child")).unwrap();
        assert!(parent.inner.lock().is_mounted("child"));

        let umounted = parent.umount_at("child").unwrap();
        assert!(!parent.inner.lock().is_mounted("child"));
        assert!(Arc::ptr_eq(&umounted, &child));
    }

    #[test]
    fn test_directory_tree_node_open_child() {
        let parent = DirectoryTreeNode::from_empty(None, "parent".to_string());
        let child = DirectoryTreeNode::from_empty(Some(parent.clone()), "child".to_string());
        parent.mount_as(child.clone(), Some("child")).unwrap();

        let opened = parent.open_child("child").unwrap();
        assert!(Arc::ptr_eq(&opened, &child));
    }

    #[test]
    fn test_directory_tree_node_fullpath() {
        let root = DirectoryTreeNode::from_empty(None, "".to_string());
        let parent = DirectoryTreeNode::from_empty(Some(root.clone()), "parent".to_string());
        let child = DirectoryTreeNode::from_empty(Some(parent.clone()), "child".to_string());

        assert_eq!(root.fullpath(), "/");
        assert_eq!(parent.fullpath(), "/parent");
        assert_eq!(child.fullpath(), "/parent/child");
    }

    #[test]
    fn test_directory_tree_node_read_dir() {
        let parent = DirectoryTreeNode::from_empty(None, "parent".to_string());
        let child = DirectoryTreeNode::from_empty(Some(parent.clone()), "child".to_string());
        parent.mount_as(child.clone(), Some("child")).unwrap();

        let entries = parent.read_dir().unwrap();
        assert!(entries.iter().any(|e| e.filename == "child"));
        assert!(entries.iter().any(|e| e.filename == "."));
        assert!(!entries.iter().any(|e| e.filename == "..")); // `parent` does not have a parent
    }

    #[test]
    fn test_directory_tree_node_read_dir_with_parent() {
        let root = DirectoryTreeNode::from_empty(None, String::new());
        let parent = DirectoryTreeNode::from_empty(Some(root), "parent".to_string());
        let child = DirectoryTreeNode::from_empty(Some(parent.clone()), "child".to_string());
        parent.mount_as(child.clone(), Some("child")).unwrap();

        let entries = parent.read_dir().unwrap();
        assert!(entries.iter().any(|e| e.filename == "child"));
        assert!(entries.iter().any(|e| e.filename == "."));
        assert!(entries.iter().any(|e| e.filename == ".."));
    }

    #[test]
    fn test_initialize() {
        initialize();
        let root = unsafe { ROOT.lock().assume_init_ref().clone() };
        let expected_dirs = [
            "boot", "dev", "etc", "home", "root", "opt", "mnt", "sys", "tmp", "run", "usr", "var",
            "bin", "proc",
        ];

        for dir in expected_dirs {
            assert!(root.inner.lock().is_mounted(dir));
        }

        assert!(root.open("/dev/tty").is_ok());
        assert!(root.open("/dev/null").is_ok());
        assert!(root.open("/dev/zero").is_ok());
        assert!(root.open("/dev/random").is_ok());
        assert!(root.open("/dev/urandom").is_ok());
    }

    #[test]
    fn test_global_open() {
        initialize();
        let root = unsafe { ROOT.lock().assume_init_ref().clone() };
        let path = "/dev/tty";
        let result = global_open(path, Some(&root));
        assert!(result.is_ok());
    }

    #[test]
    fn test_global_umount() {
        initialize();
        let root = unsafe { ROOT.lock().assume_init_ref().clone() };
        let path = "/dev/tty";
        let result = global_umount(path, Some(&root));
        assert!(result.is_ok());
    }

    #[test]
    fn test_global_mount_filesystem() {
        initialize();
        let root = unsafe { ROOT.lock().assume_init_ref().clone() };
        let fs = Arc::new(MockFileSystem);
        let path = "/mnt/test_fs";
        let result = global_mount_filesystem(fs, path, Some(&root));
        assert!(result.is_ok());
    }

    #[test]
    fn test_global_mount() {
        initialize();
        let root = unsafe { ROOT.lock().assume_init_ref().clone() };
        let node = DirectoryTreeNode::from_empty(Some(root.clone()), "test_node".to_string());
        let path = "/test_node";
        let result = global_mount(&node, path, Some(&root));
        assert!(result.is_ok());
    }

    #[test]
    fn test_global_mount_inode() {
        initialize();
        let root = unsafe { ROOT.lock().assume_init_ref().clone() };
        let inode: Arc<dyn IInode> = Arc::new(RamFileInode::new("test_inode"));
        let path = "/test_inode";
        let result = global_mount_inode(&inode, path, Some(&root));
        assert!(result.is_ok());
    }

    struct MockFileSystem;

    impl IFileSystem for MockFileSystem {
        fn name(&self) -> &str {
            "MockFileSystem"
        }

        fn root_dir(&self) -> Arc<dyn IInode> {
            unimplemented!()
        }
    }

    struct MockInode {
        name: String,
        size: usize,
    }

    impl MockInode {
        fn new(name: &str, size: usize) -> Self {
            MockInode {
                name: name.to_string(),
                size,
            }
        }
    }

    impl IInode for MockInode {
        fn metadata(&self) -> InodeMetadata {
            InodeMetadata {
                filename: &self.name,
                entry_type: DirectoryEntryType::File,
                size: self.size,
            }
        }

        fn writeat(&self, _offset: usize, _buffer: &[u8]) -> FileSystemResult<usize> {
            Ok(0)
        }

        fn readat(&self, _offset: usize, _buffer: &mut [u8]) -> FileSystemResult<usize> {
            Ok(0)
        }

        fn stat(&self, stat: &mut FileStatistics) -> FileSystemResult<()> {
            stat.mode = crate::FileStatisticsMode::FILE;
            stat.link_count = 1;
            stat.size = self.size as u64;
            stat.block_size = 4096;
            stat.block_count = 1;

            Ok(())
        }

        fn read_cache_dir(
            &self,
            _cache: &mut BTreeMap<String, Arc<dyn IInode>>,
        ) -> FileSystemResult<Vec<DirectoryEntry>> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn test_directory_tree_node_shadow_with() {
        let parent = None;
        let name = "test_dir";
        let node1 = DirectoryTreeNode::from_empty(parent.clone(), name.to_string());
        let node2 = DirectoryTreeNode::from_empty(parent, name.to_string());

        let result = Arc::clone(&node1).shadow_with(Arc::clone(&node2));
        assert!(result);
    }

    #[test]
    fn test_directory_tree_node_open_as_file() {
        let parent = None;
        let name = "test_file";
        let inode: Arc<dyn IInode> = Arc::new(MockInode::new(name, 0));
        let node = DirectoryTreeNode::from_inode(parent, &inode, Some(name));
        let opened = node.open_as_file(OpenFlags::empty(), 0);
        assert!(opened.metadata().is_some());
    }

    #[test]
    fn test_directory_tree_node_mount_as() {
        let parent = DirectoryTreeNode::from_empty(None, "parent".to_string());
        let child = DirectoryTreeNode::from_empty(Some(Arc::clone(&parent)), "child".to_string());

        let result = parent.mount_as(Arc::clone(&child), Some("child"));
        assert!(result.is_ok());
        assert!(parent.inner.lock().is_mounted("child"));
    }

    #[test]
    fn test_directory_tree_node_umount_at() {
        let parent = DirectoryTreeNode::from_empty(None, "parent".to_string());
        let child = DirectoryTreeNode::from_empty(Some(Arc::clone(&parent)), "child".to_string());

        parent.mount_as(Arc::clone(&child), Some("child")).unwrap();
        let result = parent.umount_at("child");
        assert!(result.is_ok());
        assert!(!parent.inner.lock().is_mounted("child"));
    }

    #[test]
    fn test_directory_tree_node_close() {
        let parent = DirectoryTreeNode::from_empty(None, "parent".to_string());
        let child = DirectoryTreeNode::from_empty(Some(Arc::clone(&parent)), "child".to_string());

        parent.mount_as(Arc::clone(&child), Some("child")).unwrap();
        let (closed, unmounted) = parent.close("child");
        assert!(closed || unmounted);
        assert!(!parent.inner.lock().is_mounted("child"));
    }

    #[test]
    fn test_directory_tree_node_open() {
        let parent = DirectoryTreeNode::from_empty(None, "parent".to_string());
        let child = DirectoryTreeNode::from_empty(Some(Arc::clone(&parent)), "child".to_string());

        parent.mount_as(Arc::clone(&child), Some("child")).unwrap();
        let result = parent.open("child");
        assert!(result.is_ok());
    }

    #[test]
    fn test_directory_tree_node_get_common_parent() {
        let root = DirectoryTreeNode::from_empty(None, "".to_string());
        let parent1 = DirectoryTreeNode::from_empty(Some(Arc::clone(&root)), "parent1".to_string());
        let parent2 = DirectoryTreeNode::from_empty(Some(Arc::clone(&root)), "parent2".to_string());
        let child1 =
            DirectoryTreeNode::from_empty(Some(Arc::clone(&parent1)), "child1".to_string());
        let child2 =
            DirectoryTreeNode::from_empty(Some(Arc::clone(&parent2)), "child2".to_string());

        let common_parent = DirectoryTreeNode::get_common_parent(&child1, &child2);
        assert!(Arc::ptr_eq(&common_parent, &root));
    }

    #[test]
    fn test_directory_tree_node_get_containing_filesystem() {
        let root = DirectoryTreeNode::from_empty(None, "".to_string());
        let node = DirectoryTreeNode::from_empty(Some(Arc::clone(&root)), "node".to_string());

        let fs = node.get_containing_filesystem();
        assert!(Arc::ptr_eq(&fs, &root));
    }

    #[test]
    fn test_directory_tree_node_readall() {
        let parent = None;
        let name = "test_file";
        let inode: Arc<dyn IInode> = Arc::new(MockInode::new(name, 0));
        let node = DirectoryTreeNode::from_inode(parent, &inode, Some(name));
        let result = node.readall();
        assert!(result.is_ok());
    }

    #[test]
    fn test_directory_tree_node_readrest_at() {
        let parent = None;
        let name = "test_file";
        let inode: Arc<dyn IInode> = Arc::new(MockInode::new(name, 0));
        let node = DirectoryTreeNode::from_inode(parent, &inode, Some(name));
        let result = node.readrest_at(0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_directory_tree_node_readvec_at() {
        let parent = None;
        let name = "test_file";
        let inode: Arc<dyn IInode> = Arc::new(MockInode::new(name, 0));
        let node = DirectoryTreeNode::from_inode(parent, &inode, Some(name));
        let result = node.readvec_at(0, 10);
        assert!(result.is_ok());
    }

    #[test]
    fn test_directory_tree_node_metadata() {
        let parent = None;
        let name = "test_file";
        let inode: Arc<dyn IInode> = Arc::new(MockInode::new(name, 10));
        let node = DirectoryTreeNode::from_inode(parent, &inode, Some(name));
        let meta = node.metadata();
        assert_eq!(meta.filename, name);
        assert_eq!(meta.entry_type, DirectoryEntryType::File);
        assert_eq!(meta.size, 10);
    }

    #[test]
    fn test_directory_tree_node_readat() {
        let parent = None;
        let name = "test_file";
        let inode: Arc<dyn IInode> = Arc::new(MockInode::new(name, 0));
        let node = DirectoryTreeNode::from_inode(parent, &inode, Some(name));
        let mut buffer = [0; 10];
        let result = node.readat(0, &mut buffer);
        assert!(result.is_ok());
    }

    #[test]
    fn test_directory_tree_node_writeat() {
        let parent = None;
        let name = "test_file";
        let inode: Arc<dyn IInode> = Arc::new(MockInode::new(name, 0));
        let node = DirectoryTreeNode::from_inode(parent, &inode, Some(name));
        let data = [1; 10];
        let result = node.writeat(0, &data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_directory_tree_node_mkdir() {
        let parent = DirectoryTreeNode::from_empty(None, "parent".to_string());
        let result = parent.mkdir("new_dir");
        assert!(result.is_ok());
        assert!(parent.inner.lock().is_mounted("new_dir"));
    }

    #[test]
    fn test_directory_tree_node_rmdir() {
        let parent = DirectoryTreeNode::from_empty(None, "parent".to_string());
        parent.mkdir("new_dir").unwrap();
        let result = parent.rmdir("new_dir");
        assert!(result.is_ok());
        assert!(!parent.inner.lock().is_mounted("new_dir"));
    }

    #[test]
    fn test_directory_tree_node_remove() {
        let parent = DirectoryTreeNode::from_empty(None, "parent".to_string());
        let inode: Arc<dyn IInode> = Arc::new(MockInode::new("test_file", 0));
        let child =
            DirectoryTreeNode::from_inode(Some(Arc::clone(&parent)), &inode, Some("test_file"));
        parent
            .mount_as(Arc::clone(&child), Some("test_file"))
            .unwrap();
        let result = parent.remove("test_file");
        assert!(result.is_ok());
        assert!(!parent.inner.lock().is_mounted("test_file"));
    }

    #[test]
    fn test_directory_tree_node_touch() {
        let parent = DirectoryTreeNode::from_empty(None, "parent".to_string());
        let result = parent.touch("new_file");
        assert!(result.is_ok());
        assert!(parent.inner.lock().is_mounted("new_file"));
    }

    #[test]
    fn test_directory_tree_node_stat() {
        let parent = None;
        let name = "test_file";
        let inode: Arc<dyn IInode> = Arc::new(MockInode::new(name, 10));
        let node = DirectoryTreeNode::from_inode(parent, &inode, Some(name));
        let mut stat = unsafe { core::mem::zeroed::<FileStatistics>() };
        let result = node.stat(&mut stat);
        assert!(result.is_ok());
        assert_eq!(stat.size, 10);
    }

    #[test]
    fn test_directory_tree_node_hard_link() {
        let parent = DirectoryTreeNode::from_empty(None, "parent".to_string());
        let inode: Arc<dyn IInode> = Arc::new(MockInode::new("test_file", 0));
        let source =
            DirectoryTreeNode::from_inode(Some(Arc::clone(&parent)), &inode, Some("test_file"));
        let result = parent.hard_link("link_file", &source);
        assert!(result.is_ok());
        assert!(parent.inner.lock().is_mounted("link_file"));
    }

    #[test]
    fn test_directory_tree_node_soft_link() {
        let parent = DirectoryTreeNode::from_empty(None, "parent".to_string());
        let result = parent.soft_link("soft_link", "target");
        assert!(result.is_ok());
        assert!(parent.inner.lock().is_mounted("soft_link"));
    }
}
