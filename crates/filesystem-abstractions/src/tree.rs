use address::{IPageNum, PhysicalAddress};
use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::{String, ToString},
    sync::{Arc, Weak},
    vec::Vec,
};
use allocation::TrackedFrame;
use constants::SyscallError;
use core::{mem::MaybeUninit, usize};
use hermit_sync::{RwSpinLock, SpinMutex};
use timing::TimeSpec;

use crate::{
    special_inode::{RandomInode, UnblockedRandomInode},
    DirectoryEntry, DirectoryEntryType, FileMetadata, FileStatistics, FileStatisticsMode,
    FileSystemError, FileSystemResult, IFileSystem, IInode, InodeMetadata, NullInode, OpenFlags,
    OpenedDiskInode, TeleTypewriterInode, ZeroInode,
};

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
        let mut inner = self.inner.write();

        let end_size = offset + buffer.len();

        if end_size > inner.size {
            let required_pages = (end_size + 4095) / 4096;
            inner.frames.resize_with(required_pages, || {
                // TODO: do we have to zero the memory?
                allocation::alloc_frame().expect("Out of memory")
            });
            inner.size = end_size;
        }

        let mut current = offset;
        for frame in &inner.frames[offset / 4096..end_size / 4096 + 1] {
            let in_page_start = current % 4096;
            let in_page_len = usize::min(4096, end_size - current);

            let data_ptr = unsafe {
                frame
                    .ppn()
                    .start_addr::<PhysicalAddress>()
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

            let data_ptr = unsafe {
                frame
                    .ppn()
                    .start_addr::<PhysicalAddress>()
                    .to_high_virtual()
                    .as_ptr::<u8>()
            };
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

#[derive(Clone)]
struct DirectoryTreeNodeInner {
    meta: DirectoryTreeNodeMetadata,
    name: String,
    mounted: BTreeMap<String, Arc<DirectoryTreeNode>>,
    opened: BTreeMap<String, Weak<DirectoryTreeNode>>,
    children_cache: BTreeMap<String, Arc<dyn IInode>>,
    shadowed: Option<Arc<DirectoryTreeNode>>,
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
    pub fn shadow_with(self: &Arc<DirectoryTreeNode>, new: &Arc<DirectoryTreeNode>) {
        if Arc::ptr_eq(self, new) {
            return;
        }

        let mut node_inner = self.inner.lock();

        let previous = Arc::new(DirectoryTreeNode {
            parent: None, // Not needed
            inner: SpinMutex::new(node_inner.clone()),
        });

        *node_inner = new.inner.lock().clone();
        node_inner.shadowed = Some(previous);
    }

    pub fn restore_shadow(self: &Arc<DirectoryTreeNode>) {
        let mut inner = self.inner.lock();

        let shadowed = inner.shadowed.clone();
        if let Some(shadowed) = shadowed {
            debug_assert!(!Arc::ptr_eq(self, &shadowed));

            *inner = shadowed.inner.lock().clone();
        }

        inner.shadowed = None;
    }
}

impl DirectoryTreeNode {
    pub fn open_as_file(
        self: Arc<DirectoryTreeNode>,
        flags: OpenFlags,
        offset: usize,
    ) -> Arc<OpenedDiskInode> {
        Arc::new(OpenedDiskInode {
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
                if let DirectoryTreeNodeMetadata::Inode { inode } =
                    unsafe { &node.inner.data_ptr().as_ref().unwrap().meta }
                {
                    return inode.metadata().filename;
                }

                node.metadata().filename
            }

            log::info!(
                "Mounting {} at {}/{}",
                get_raw_name(&node),
                self.fullpath(),
                name
            );
        }

        if let Some(mounted) = self.inner.lock().mounted.remove(name) {
            let mut new_inner = node.inner.lock();

            new_inner.shadowed = Some(mounted);
        }

        self.inner
            .lock()
            .mounted
            .insert(name.to_string(), node.clone())
            .map_or_else(|| Ok(node.clone()), |_| Err(MountError::FileExists))
    }

    pub fn mount_as(
        self: &Arc<DirectoryTreeNode>,
        node: Arc<DirectoryTreeNode>,
        name: Option<&str>,
    ) -> Result<Arc<DirectoryTreeNode>, MountError> {
        let name = name.unwrap_or(node.name());

        let new = if node.parent.is_none()
            || !Arc::ptr_eq(self, unsafe { node.parent.as_ref().unwrap_unchecked() })
        {
            Arc::new(DirectoryTreeNode {
                parent: Some(self.clone()),
                inner: SpinMutex::new(node.inner.lock().clone()),
            })
        } else {
            node.clone()
        };

        self.mount_internal(new, name)
    }

    pub fn mount_empty(
        self: &Arc<DirectoryTreeNode>,
        name: &str,
    ) -> Result<Arc<DirectoryTreeNode>, MountError> {
        let node = Self::from_empty(Some(self.clone()), name.to_string());

        self.mount_internal(node, name)
    }

    pub fn umount_at(&self, name: &str) -> Result<Arc<DirectoryTreeNode>, MountError> {
        let umounted = self
            .inner
            .lock()
            .mounted
            .remove(name)
            .ok_or(MountError::FileNotExists)?;

        let mut umounted_inner = umounted.inner.lock();

        if let Some(shadowed) = umounted_inner.shadowed.take() {
            let mut self_inner = self.inner.lock();

            self_inner.mounted.insert(name.to_string(), shadowed);
        }

        umounted_inner.mounted.clear(); // prevent memory leak caused by loop reference

        drop(umounted_inner);

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
            return Ok(self.clone());
        }

        if name == path::PARENT_DIRECTORY {
            return self.parent.as_ref().map_or_else(
                || Ok(self.clone()),
                |parent: &Arc<DirectoryTreeNode>| Ok(parent.clone()),
            );
        }

        // prevent dead lock in lookup method
        {
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

            if !inner.children_cache.is_empty() {
                if let Some(inode) = inner.children_cache.remove(name) {
                    return Ok(Self::from_inode(Some(self.clone()), &inode, Some(name)));
                }
            }
        }

        #[allow(deprecated)]
        let inode = self.lookup(name)?;

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

        let mut current = self.clone();
        stack.push(current.name_internal());

        while let Some(parent) = &current.parent {
            current = parent.clone();
            stack.push(current.name_internal());
        }

        let size = stack.iter().map(|s| s.len()).sum::<usize>() + stack.len();
        let mut path = String::with_capacity(size);

        while let Some(part) = stack.pop() {
            path.push_str(part);

            if !stack.is_empty() {
                path.push_str(path::SEPARATOR_STR);
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

    fn lookup(self: &Arc<DirectoryTreeNode>, name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        // We dont't use DirectoryTreeNode::open because this method only cares the lookup process,
        // it doesn't mean the inode has to be opened.
        let inner = self.inner.lock();

        #[allow(deprecated)]
        match inner.meta.as_inode() {
            Some(inode) => inode.lookup(name),
            None => Err(FileSystemError::NotFound),
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

        match inner.meta.as_inode() {
            Some(inode) => {
                let made = inode.mkdir(name)?;

                let wrapped = Self::from_inode(Some(self.clone()), &made, None);

                inner
                    .opened
                    .insert(wrapped.name().to_string(), Arc::downgrade(&wrapped));

                Ok(wrapped)
            }
            None => {
                drop(inner); // release lock, as mount operation requires lock

                self.mount_empty(name).map_err(|e| e.to_filesystem_error())
            }
        }
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

        match inner.meta.as_inode() {
            Some(inode) => {
                let touched = inode.touch(name)?;

                let wrapped = DirectoryTreeNode::from_inode(Some(self.clone()), &touched, None);

                inner
                    .opened
                    .insert(wrapped.name().to_string(), Arc::downgrade(&wrapped));

                Ok(wrapped)
            }
            None => {
                drop(inner); // release lock, as mount operation requires lock

                let ram_inode: Arc<dyn IInode> = Arc::new(RamFileInode::new(name));

                global_mount_inode(&ram_inode, name, Some(self))
                    .map_err(|e| e.to_filesystem_error())
            }
        }
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
                    let inode = inode.clone();

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
        let under_same_filesystem = Arc::ptr_eq(
            &self.get_containing_filesystem(),
            &source.get_containing_filesystem(),
        );

        let self_inner = self.inner.lock();
        let source_inner = source.inner.lock();

        match (
            under_same_filesystem,
            self_inner.meta.as_inode(),
            source_inner.meta.as_inode(),
        ) {
            (true, Some(ref self_inode), Some(ref source_inode)) => {
                self_inode.hard_link(name, source_inode)
            }
            _ => {
                drop(self_inner);
                drop(source_inner);

                self.mount_as(source.clone(), Some(name))
                    .map_err(|e| e.to_filesystem_error())?;

                Ok(())
            }
        }
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
                Some(target) => match global_open_raw(&target, Some(&current)) {
                    Ok(node) => current = node,
                    Err(_) => return Err(FileSystemError::NotFound),
                },
            }
        }

        Err(FileSystemError::LinkTooDepth)
    }
}

// The root of the directory tree
static mut ROOT: SpinMutex<MaybeUninit<Arc<DirectoryTreeNode>>> =
    SpinMutex::new(MaybeUninit::uninit());

pub fn initialize() {
    let root = DirectoryTreeNode::from_empty(None, String::new());

    for node in [
        "boot", "dev", "etc", "home", "root", "opt", "mnt", "sys", "tmp", "run", "usr", "var",
        "bin",
    ]
    .iter()
    {
        root.mount_empty(node).unwrap();
    }

    unsafe {
        *ROOT.lock() = MaybeUninit::new(root);
    }

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
        current = current.open_child(part)?;
    }

    Ok(current)
}

pub fn global_umount(
    path: &str,
    relative_to: Option<&Arc<DirectoryTreeNode>>,
) -> Result<Arc<DirectoryTreeNode>, MountError> {
    let root = match (relative_to, path::is_path_fully_qualified(path)) {
        (_, true) => {
            let mut root = unsafe { ROOT.lock() };

            let root_node = unsafe { root.assume_init_ref() };

            // Umount root, restoring shadowed node
            if path.trim_start_matches(path::SEPARATOR).is_empty() {
                let mut root_inner = root_node.inner.lock();
                let previous_root = root_inner
                    .shadowed
                    .take()
                    .unwrap_or(DirectoryTreeNode::from_empty(None, String::new()));

                root_inner.mounted.clear(); // prevent memory leak cuz the mounted nodes and root hold reference to each other

                drop(root_inner);

                *root = MaybeUninit::new(previous_root.clone());

                return Ok(previous_root);
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
            let mut root = unsafe { ROOT.lock() };

            // new root
            if path.trim_start_matches(path::SEPARATOR).is_empty() {
                let new_root = get_node(None, "");
                new_root.inner.lock().shadowed = Some(unsafe { root.assume_init_ref().clone() });

                *root = MaybeUninit::new(new_root.clone());

                return Ok(new_root);
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
