use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    sync::Weak,
    vec::Vec,
};
use constants::SyscallError;
use core::{cell::UnsafeCell, usize};
use hermit_sync::SpinMutex;

use crate::{
    DirectoryEntry, DirectoryEntryType, FileStatistics, FileSystemError, FileSystemResult, IInode,
    InodeMetadata,
};

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

enum DirectoryTreeNodeMetadata {
    Inode { inode: Arc<dyn IInode> },
    Empty,
}

struct DirectoryTreeNodeInner {
    meta: DirectoryTreeNodeMetadata,
    name: String,
    mounted: BTreeMap<String, Arc<DirectoryTreeNode>>,
    opened: BTreeMap<String, Weak<DirectoryTreeNode>>,
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
    weak_self: UnsafeCell<Weak<DirectoryTreeNode>>,
    inner: Arc<SpinMutex<DirectoryTreeNodeInner>>,
}

unsafe impl Send for DirectoryTreeNode {}
unsafe impl Sync for DirectoryTreeNode {}

impl DirectoryTreeNode {
    fn set_weak(self: &Arc<DirectoryTreeNode>) {
        unsafe { *self.weak_self.get().as_mut().unwrap() = Arc::downgrade(self) }
    }

    fn self_arc(&self) -> Option<Arc<DirectoryTreeNode>> {
        let weak = unsafe { self.weak_self.get().as_ref() }?;

        weak.upgrade()
    }

    pub fn from_empty(
        parent: Option<Arc<DirectoryTreeNode>>,
        name: String,
    ) -> Arc<DirectoryTreeNode> {
        let arc = Arc::new(DirectoryTreeNode {
            parent,
            weak_self: UnsafeCell::new(Weak::new()),
            inner: Arc::new(SpinMutex::new(DirectoryTreeNodeInner {
                meta: DirectoryTreeNodeMetadata::Empty,
                name,
                mounted: BTreeMap::new(),
                opened: BTreeMap::new(),
            })),
        });

        arc.set_weak();

        arc
    }

    pub fn from_inode(
        parent: Option<Arc<DirectoryTreeNode>>,
        inode: &Arc<dyn IInode>,
        inode_meta: Option<&InodeMetadata>,
    ) -> Arc<DirectoryTreeNode> {
        let arc = Arc::new(DirectoryTreeNode {
            parent,
            weak_self: UnsafeCell::new(Weak::new()),
            inner: Arc::new(SpinMutex::new(DirectoryTreeNodeInner {
                meta: DirectoryTreeNodeMetadata::Inode {
                    inode: inode.clone(),
                },
                name: inode_meta
                    .map(|m| m.filename.to_string())
                    .unwrap_or_default(),
                mounted: BTreeMap::new(),
                opened: BTreeMap::new(),
            })),
        });

        arc.set_weak();

        arc
    }

    pub fn mount_as(
        self: &Arc<DirectoryTreeNode>,
        inode: &Arc<dyn IInode>,
        name: Option<&str>,
    ) -> Result<Arc<dyn IInode>, MountError> {
        let mut inner = self.inner.lock();

        let name = match name {
            Some(n) => n,
            None => {
                let inode_meta = inode.metadata().map_err(|_| MountError::InvalidInput)?;

                inode_meta.filename
            }
        };

        if inner.is_mounted(name) {
            return Err(MountError::FileExists);
        }

        // TODO: Figure out whether we should have a directory stub to mount
        // let is_existed = match &inner.meta {
        //     DirectoryTreeNodeMetadata::Inode {
        //         name: _,
        //         inode: this,
        //     } => {
        //         let this_meta = inode
        //             .metadata()
        //             .map_err(|_| MountError::MetadataUnavailable)?;

        //         if this_meta.entry_type != DirectoryEntryType::Directory {
        //             return Err(MountError::NotADirectory);
        //         }

        //         this.lookup(inode_meta.filename).is_ok()
        //     }
        //     DirectoryTreeNodeMetadata::Empty { name: _ } => false, // already checked
        // };

        // if !is_existed {
        //     return Err(MountError::FileNotExists);
        // }

        // We actually don't care what the name of the inode to be mounted is,
        // as the 'mount' operation always gives a new name to it, which is the key of the mount list
        let inode = Self::from_inode(Some(self.clone()), inode, inode.metadata().as_ref().ok());

        inner
            .mounted
            .insert(name.to_string(), inode)
            .map(|n| n as Arc<dyn IInode>)
            .ok_or(MountError::FileExists)
    }

    pub fn umount_at(&self, name: &str) -> Result<Arc<DirectoryTreeNode>, MountError> {
        self.inner
            .lock()
            .mounted
            .remove(name)
            .ok_or(MountError::FileNotExists)
    }

    pub fn name(&self) -> &str {
        self.name_internal()
    }

    fn name_internal(&self) -> &'static str {
        unsafe { &self.inner.data_ptr().as_ref().unwrap().name }
    }

    pub fn close(&self, name: &str) -> (bool, bool) {
        let mut inner = self.inner.lock();

        let closed = inner.opened.remove(name).is_some();
        let unmounted = inner.mounted.remove(name).is_some();

        (closed, unmounted)
    }

    pub fn open(
        self: &Arc<DirectoryTreeNode>,
        name: &str,
    ) -> FileSystemResult<Arc<DirectoryTreeNode>> {
        // prevent dead lock in lookup method
        {
            let inner = self.inner.lock();

            if let Some(opened) = inner.opened.get(name).and_then(|weak| weak.upgrade()) {
                return Ok(opened);
            }

            if let Some(mounted) = inner.mounted.get(name).cloned() {
                return Ok(mounted);
            }
        }

        #[allow(deprecated)]
        let inode = self.lookup(name)?;
        let inode_meta = inode.as_ref().metadata()?;

        Ok(Self::from_inode(
            Some(self.clone()),
            &inode,
            Some(&inode_meta),
        ))
    }

    // if the node was opened in the tree, this returns the full path in the filesystem.
    // if not, the root is considered the deepest node without parent
    pub fn fullpath(&self) -> String {
        let mut stack = Vec::new();

        let mut current = self.self_arc().expect("Unable to get self arc");
        stack.push(current.name_internal());

        while let Some(parent) = &current.parent {
            current = parent.clone();
            stack.push(current.name_internal());
        }

        let size = stack.iter().map(|s| s.len()).sum::<usize>() + stack.len();
        let mut path = String::with_capacity(size);

        while let Some(part) = stack.pop() {
            path.push_str(path::SEPARATOR_STR);
            path.push_str(part);
        }

        path
    }
}

impl Drop for DirectoryTreeNode {
    fn drop(&mut self) {
        if let Some(ref parent) = self.parent {
            parent.close(self.name());
        }
    }
}

impl IInode for DirectoryTreeNode {
    fn metadata(&self) -> FileSystemResult<InodeMetadata> {
        let inner = self.inner.lock();
        let filename = self.name();

        match &inner.meta {
            DirectoryTreeNodeMetadata::Inode { inode } => {
                let meta = inode.metadata()?;

                Ok(InodeMetadata {
                    filename,
                    entry_type: meta.entry_type,
                    size: meta.size,
                    children_count: meta.children_count,
                })
            }
            DirectoryTreeNodeMetadata::Empty => Ok(InodeMetadata {
                filename,
                entry_type: DirectoryEntryType::Directory,
                size: 0,
                children_count: inner.mounted.len(),
            }),
        }
    }

    fn lookup(&self, name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        // We dont't use DirectoryTreeNode::open because this method only cares the lookup process,
        // it doesn't mean the inode has to be opened.
        let inner = self.inner.lock();

        if let Some(inode) = inner.get_mounted(name) {
            return Ok(inode);
        }

        if let Some(opened) = inner.opened.get(name).and_then(|weak| weak.upgrade()) {
            return Ok(opened);
        }

        #[allow(deprecated)]
        match &inner.meta {
            DirectoryTreeNodeMetadata::Inode { inode } => inode.lookup(name),
            DirectoryTreeNodeMetadata::Empty => Err(FileSystemError::NotFound),
        }
    }

    fn readat(&self, offset: usize, buffer: &mut [u8]) -> FileSystemResult<usize> {
        match &self.inner.lock().meta {
            DirectoryTreeNodeMetadata::Inode { inode } => inode.readat(offset, buffer),
            DirectoryTreeNodeMetadata::Empty => Err(FileSystemError::NotAFile),
        }
    }

    fn writeat(&self, offset: usize, buffer: &[u8]) -> FileSystemResult<usize> {
        match &self.inner.lock().meta {
            DirectoryTreeNodeMetadata::Inode { inode } => inode.writeat(offset, buffer),
            DirectoryTreeNodeMetadata::Empty => Err(FileSystemError::NotAFile),
        }
    }

    fn mkdir(&self, name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        let inner = self.inner.lock();

        if inner.is_mounted(name) {
            return Err(FileSystemError::AlreadyExists);
        }

        match &inner.meta {
            DirectoryTreeNodeMetadata::Inode { inode } => inode.mkdir(name),
            DirectoryTreeNodeMetadata::Empty => {
                let self_arc = self.self_arc().expect("Unable to get self arc");
                let inode = Self::from_empty(Some(self_arc.clone()), String::from(name));

                self_arc
                    .mount_as(&(inode as Arc<dyn IInode>), Some(name))
                    .map_err(|e| e.to_filesystem_error())
            }
        }
    }

    fn rmdir(&self, name: &str) -> FileSystemResult<()> {
        // FIXME: Do we have to check if it's a directory?
        if self.close(name).1 {
            return Ok(());
        }

        match &self.inner.lock().meta {
            DirectoryTreeNodeMetadata::Inode { inode } => inode.rmdir(name),
            DirectoryTreeNodeMetadata::Empty => Ok(()), // same as below
        }
    }

    fn remove(&self, name: &str) -> FileSystemResult<()> {
        if self.close(name).1 {
            return Ok(());
        }

        match &self.inner.lock().meta {
            DirectoryTreeNodeMetadata::Inode { inode } => inode.remove(name),
            DirectoryTreeNodeMetadata::Empty => Ok(()), // already removed in close method
        }
    }

    fn touch(&self, name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        match &self.inner.lock().meta {
            DirectoryTreeNodeMetadata::Inode { inode } => inode.touch(name),
            DirectoryTreeNodeMetadata::Empty => todo!("Implement ram inode"),
        }
    }

    fn read_dir(&self) -> FileSystemResult<Vec<DirectoryEntry>> {
        let inner = self.inner.lock();
        let mounted_entries =
            inner
                .mounted
                .iter()
                .map(|(name, mounted)| match &mounted.inner.lock().meta {
                    DirectoryTreeNodeMetadata::Inode { inode } => {
                        let inode_meta = inode.metadata().expect("Mounted node with no metadata");

                        DirectoryEntry {
                            filename: name.clone(),
                            size: inode_meta.size,
                            entry_type: inode_meta.entry_type,
                        }
                    }
                    DirectoryTreeNodeMetadata::Empty => DirectoryEntry {
                        filename: name.clone(),
                        size: 0,
                        entry_type: DirectoryEntryType::Directory,
                    },
                });

        match &inner.meta {
            DirectoryTreeNodeMetadata::Inode { inode } => {
                let mut entries = inode.read_dir()?;

                for entry in mounted_entries {
                    entries.push(entry);
                }

                Ok(entries)
            }
            DirectoryTreeNodeMetadata::Empty => Ok(mounted_entries.collect()),
        }
    }

    fn stat(&self, stat: &mut FileStatistics) -> FileSystemResult<()> {
        match &self.inner.lock().meta {
            DirectoryTreeNodeMetadata::Inode { inode } => inode.stat(stat),
            DirectoryTreeNodeMetadata::Empty => Err(FileSystemError::Unimplemented), // TODO: do we have to implement this?
        }
    }
}

// The root of the directory tree
static mut ROOT: SpinMutex<Option<Arc<DirectoryTreeNode>>> = SpinMutex::new(None);

pub fn global_open(
    path: &str,
    relative_to: Option<&Arc<DirectoryTreeNode>>,
) -> FileSystemResult<Arc<DirectoryTreeNode>> {
    let root = match (relative_to, path::is_path_fully_qualified(path)) {
        (_, true) => unsafe {
            ROOT.lock()
                .as_ref()
                .cloned()
                .ok_or(FileSystemError::NotFound)?
        },
        (Some(root), false) => root.clone(),
        (None, false) => return Err(FileSystemError::InvalidInput),
    };

    let parts = path.split('/').skip_while(|d| d.is_empty());

    let mut current = root;
    for part in parts {
        current = current.open(part)?;
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

            match root.as_ref().cloned() {
                Some(root) => root,
                None => {
                    // Umount root
                    if path.trim_start_matches('/').is_empty() {
                        match root.take() {
                            Some(r) => {
                                let node = r;
                                *root = None;
                                return Ok(node);
                            }
                            None => return Err(MountError::FileNotExists),
                        }
                    }

                    return Err(MountError::InvalidInput);
                }
            }
        }
        (Some(root), false) => root.clone(),
        (None, false) => return Err(MountError::InvalidInput),
    };

    let parent_path = path::get_directory_name(path).unwrap_or("");
    let name = path::get_filename(path);

    let parent = global_open(parent_path, Some(&root)).map_err(|_| MountError::FileNotExists)?;

    parent.umount_at(name)
}

pub fn global_mount(
    inode: &Arc<dyn IInode>,
    path: &str,
    relative_to: Option<&Arc<DirectoryTreeNode>>,
) -> Result<Arc<dyn IInode>, MountError> {
    let root = match (relative_to, path::is_path_fully_qualified(path)) {
        (_, true) => {
            let mut root = unsafe { ROOT.lock() };
            match root.as_ref().cloned() {
                Some(root) => root,
                None => {
                    // Mount root
                    if path.trim_start_matches('/').is_empty() {
                        let node = DirectoryTreeNode::from_inode(None, inode, None);
                        *root = Some(node.clone());
                        return Ok(node);
                    }

                    return Err(MountError::InvalidInput);
                }
            }
        }
        (Some(root), false) => root.clone(),
        (None, false) => return Err(MountError::InvalidInput),
    };

    let parent_path = path::get_directory_name(path).unwrap_or("");
    let name = path::get_filename(path);

    let parent = global_open(parent_path, Some(&root)).map_err(|_| MountError::FileNotExists)?;

    parent.mount_as(inode, Some(name))
}
