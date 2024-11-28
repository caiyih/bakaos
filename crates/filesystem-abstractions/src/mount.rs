use core::cell::UnsafeCell;

use alloc::{string::String, sync::Arc, vec::Vec};
use hermit_sync::RwSpinLock;
use log::debug;

use crate::{IFileSystem, IInode};

struct MountPoint {
    pub fs: Arc<dyn IFileSystem + 'static>,
    pub mount_path: String,
}

static mut MOUNT_TABLE: Option<RwSpinLock<UnsafeCell<MountTable>>> = None;

pub fn mount_at(fs: Arc<dyn IFileSystem>, path: &str) -> usize {
    debug!("Mounting filesystem {} at {}", fs.name(), path);
    unsafe {
        let mount_table =
            MOUNT_TABLE.get_or_insert_with(|| RwSpinLock::new(UnsafeCell::new(MountTable::new())));
        let mount_table = mount_table.write();
        mount_table.get().as_mut().unwrap().mount(fs, path)
    }
}

pub fn umount(mount_point_id: usize) -> bool {
    unsafe {
        let mount_table =
            MOUNT_TABLE.get_or_insert_with(|| RwSpinLock::new(UnsafeCell::new(MountTable::new())));
        let mount_table = mount_table.write();
        mount_table.get().as_mut().unwrap().umount(mount_point_id)
    }
}

pub fn lookup_inode(path: &str) -> Option<Arc<dyn IInode>> {
    unsafe {
        let mount_table =
            MOUNT_TABLE.get_or_insert_with(|| RwSpinLock::new(UnsafeCell::new(MountTable::new())));
        let mount_table = mount_table.read();
        mount_table.get().as_ref().unwrap().lookup_inode(path)
    }
}

struct MountTable {
    mounts: Vec<Option<MountPoint>>,
}

impl MountTable {
    pub fn new() -> Self {
        let mounts = Vec::new();
        MountTable { mounts }
    }

    pub fn umount(&'static mut self, mount_point_id: usize) -> bool {
        match self.mounts.get_mut(mount_point_id) {
            Some(mount) => {
                *mount = None;
                true
            }
            None => false,
        }
    }

    pub fn mount(&'static mut self, fs: Arc<dyn IFileSystem>, mut path: &str) -> usize {
        if path.is_empty() {
            path = path::SEPARATOR_STR;
        }

        let mount = MountPoint {
            fs,
            mount_path: String::from(path),
        };

        let mut index = -1isize;
        for (i, m) in self.mounts.iter().enumerate() {
            if m.is_none() {
                index = i as isize;
                break;
            }
        }

        if index == -1 {
            self.mounts.push(Some(mount));
            index = self.mounts.len() as isize - 1;
        } else {
            self.mounts[index as usize] = Some(mount);
        }

        index as usize
    }

    pub fn lookup_inode(&'static self, path: &str) -> Option<Arc<dyn IInode>> {
        let path = match Self::resolve_path(path) {
            Some(path) => path,
            None => return None,
        };

        match self.get_mount_point(&path) {
            Some(mnt) => {
                let mnt = self.mounts[mnt].as_ref();
                match mnt {
                    Some(mnt) => {
                        let path = path::get_relative_path(&mnt.mount_path, &path);
                        match path {
                            Some(path) => mnt.fs.lookup_inode(&path).ok(),
                            None => None,
                        }
                    }
                    None => None,
                }
            }
            None => None,
        }
    }

    fn get_mount_point(&self, path: &str) -> Option<usize> {
        let mut max_common = 0;
        let mut mount_point_idx = None;

        for (idx, mnt) in self.mounts.iter().enumerate() {
            if let Some(mnt) = mnt.as_ref() {
                let common = path::get_common_length(&mnt.mount_path, path);
                if common > max_common {
                    max_common = common;
                    mount_point_idx = Some(idx);
                }
            }
        }

        if max_common == 0 {
            return None;
        }

        mount_point_idx
    }

    fn resolve_path(path: &str) -> Option<String> {
        if path.is_empty() || !path::is_path_fully_qualified(path) {
            return None;
        }

        Some(path::remove_relative_segments(path))
    }
}
