// use core::{
//     cell::UnsafeCell,
//     mem::{forget, MaybeUninit},
//     panic,
// };

// use alloc::sync::Arc;
// use alloc::{
//     string::{String, ToString},
//     vec::Vec,
// };
// use drivers::DiskDriver;
// use embedded_io::{Read, Seek, SeekFrom, Write};
// use filesystem_abstractions::{FileSystemError, IFileSystem, IInode};
// use hermit_sync::SpinMutex;
// use lwext4_rs::{
//     self, BlockDevice, BlockDeviceInterface, FileSystem, MetaDataExt, MountHandle, RegisterHandle,
// };

// const BLOCK_SIZE: usize = 4096;

// struct ExtFsInfo {
//     len: u64,
//     block_size: u32,
// }

// struct Lwext4Disk {
//     driver: SpinMutex<DiskDriver>,
//     fs_info: MaybeUninit<ExtFsInfo>,
// }

// unsafe impl Send for Lwext4Disk {}
// unsafe impl Sync for Lwext4Disk {}

// impl BlockDeviceInterface for Lwext4Disk {
//     fn open(&mut self) -> lwext4_rs::Result<lwext4_rs::BlockDeviceConfig> {
//         let info = unsafe { self.fs_info.assume_init_ref() };

//         Ok(lwext4_rs::BlockDeviceConfig {
//             block_size: info.block_size,
//             block_count: info.len / info.block_size as u64,
//             part_size: info.len,
//             part_offset: 0,
//         })
//     }

//     fn read_block(
//         &mut self,
//         buf: &mut [u8],
//         block_id: u64,
//         _block_count: u32,
//     ) -> lwext4_rs::Result<usize> {
//         debug_assert!(self.driver.is_locked());

//         let mut driver = unsafe { self.driver.make_guard_unchecked() };

//         unsafe {
//             driver.set_position(block_id as usize);

//             driver
//                 .read_at(buf)
//                 .map_err(|_| lwext4_rs::Error::InvalidError)
//         }
//     }

//     fn write_block(
//         &mut self,
//         mut buf: &[u8],
//         mut block_id: u64,
//         _block_count: u32,
//     ) -> lwext4_rs::Result<usize> {
//         debug_assert!(self.driver.is_locked());

//         let mut driver = unsafe { self.driver.make_guard_unchecked() };
//         let block_size = unsafe { self.fs_info.assume_init_ref().block_size } as u64;

//         while !buf.is_empty() {
//             if block_id % block_size == 0 {
//                 unsafe {
//                     driver.set_position(block_id as usize);

//                     return driver
//                         .write_at(buf)
//                         .map_err(|_| lwext4_rs::Error::InvalidError);
//                 }
//             } else {
//                 debug_assert!(block_size.is_power_of_two());
//                 debug_assert!(block_size <= 4096);

//                 let block_start = block_id & !(block_size - 1);

//                 let tmp_buf: [MaybeUninit<u8>; 4096] = [MaybeUninit::<u8>::uninit(); 4096];
//                 let mut tmp_buf = unsafe { core::mem::transmute::<_, [u8; 4096]>(tmp_buf) };

//                 unsafe {
//                     driver.set_position(block_start as usize);

//                     driver
//                         .read_at(&mut tmp_buf)
//                         .map_err(|_| lwext4_rs::Error::InvalidError)?;

//                     let offset = (block_id - block_start) as usize;
//                     let len = block_size as usize - offset;

//                     tmp_buf[offset..offset + len].copy_from_slice(&buf[..len]);

//                     driver
//                         .write_at(&tmp_buf)
//                         .map_err(|_| lwext4_rs::Error::InvalidError)?;

//                     buf = &buf[..len];
//                     block_id += len as u64;
//                 }
//             }
//         }

//         todo!()
//     }

//     fn close(&mut self) -> lwext4_rs::Result<()> {
//         assert!(!self.driver.is_locked());
//         Ok(())
//     }

//     fn lock(&mut self) -> lwext4_rs::Result<()> {
//         forget(self.driver.lock());
//         Ok(())
//     }

//     fn unlock(&mut self) -> lwext4_rs::Result<()> {
//         unsafe { self.driver.force_unlock() };
//         Ok(())
//     }
// }

// pub struct Lwext4FileSystem {
//     root_dir: Arc<Lwext4Inode>,
// }

// unsafe impl Send for Lwext4FileSystem {}
// unsafe impl Sync for Lwext4FileSystem {}

// impl Lwext4FileSystem {
//     pub fn new(device: DiskDriver) -> Self {
//         let block_device = BlockDevice::new(Lwext4Disk {
//             driver: SpinMutex::new(device),
//             fs_info: MaybeUninit::new(ExtFsInfo {
//                 len: 2 * 1024 * 1024 * 1024 * 4096,
//                 block_size: BLOCK_SIZE as u32,
//             })
//         });

//         let fs = lwext4_rs::FsBuilder::new()
//             .ty(lwext4_rs::FsType::Ext4)
//             .journal(true)
//             .block_size(BLOCK_SIZE as u32)
//             .build(block_device);

//         let _err = fs.as_ref().err().unwrap();

//         let fs = fs.unwrap();

//         let info = fs.fs_info().expect("Failed to get fs info");
//         let mut block_device = fs.take_device();

//         block_device.fs_info = MaybeUninit::new(ExtFsInfo {
//             len: info.len,
//             block_size: info.block_size,
//         });

//         let register_handle = RegisterHandle::register(block_device, String::from("/"))
//             .expect("Failed to create register handle");

//         let mount_handle = MountHandle::mount(register_handle, String::from("/"), false, false)
//             .expect("Failed to create mount handle");

//         #[allow(clippy::arc_with_non_send_sync)]
//         let inner = Arc::new(
//             lwext4_rs::FileSystem::new(mount_handle).expect("Failed to create lwext4 file system"),
//         );

//         let root_dir = Arc::new(Lwext4Inode {
//             inner: UnsafeCell::new(
//                 inner
//                     .file_builder()
//                     .open("/")
//                     .expect("Failed to open root directory"),
//             ),
//             fs: inner,
//             filename: String::from("/"),
//         });

//         Lwext4FileSystem { root_dir }
//     }
// }

// impl IFileSystem for Lwext4FileSystem {
//     fn name(&self) -> &str {
//         "Lwext4FileSystem"
//     }

//     fn root_dir(&'static self) -> Arc<dyn filesystem_abstractions::IInode> {
//         self.root_dir.clone()
//     }
// }

// struct Lwext4Inode {
//     inner: UnsafeCell<lwext4_rs::File>,
//     filename: String,
//     fs: Arc<FileSystem<Lwext4Disk>>,
// }

// impl Lwext4Inode {
//     fn inner(&self) -> &'static mut lwext4_rs::File {
//         unsafe { self.inner.get().as_mut().unwrap() }
//     }
// }

// impl Lwext4Inode {
//     fn should_be_dir(&self) -> Result<(), FileSystemError> {
//         let metadata = self
//             .inner()
//             .metadata()
//             .map_err(|_| FileSystemError::InternalError)?;

//         if metadata.is_dir() {
//             Ok(())
//         } else {
//             Err(FileSystemError::NotADirectory)
//         }
//     }

//     fn should_be_file(&self) -> Result<(), FileSystemError> {
//         let metadata = self
//             .inner()
//             .metadata()
//             .map_err(|_| FileSystemError::InternalError)?;

//         if metadata.is_file() {
//             Ok(())
//         } else {
//             Err(FileSystemError::NotAFile)
//         }
//     }
// }

// unsafe impl Send for Lwext4Inode {}
// unsafe impl Sync for Lwext4Inode {}

// impl IInode for Lwext4Inode {
//     fn flush(&self) -> filesystem_abstractions::FileSystemResult<()> {
//         Ok(())
//     }

//     fn lookup(&self, name: &str) -> filesystem_abstractions::FileSystemResult<Arc<dyn IInode>> {
//         self.should_be_dir()?;

//         let allocated: String;
//         let path = match path::is_path_fully_qualified(name) {
//             true => name,
//             false => {
//                 allocated = path::combine(&self.inner().path(), name)
//                     .ok_or(FileSystemError::InvalidInput)?;
//                 &allocated
//             }
//         };

//         let inode = self
//             .fs
//             .file_builder()
//             .open(path)
//             .map_err(|_| FileSystemError::NotFound)?;

//         Ok(Arc::new(Lwext4Inode {
//             inner: UnsafeCell::new(inode),
//             filename: path::get_filename(name).to_string(),
//             fs: self.fs.clone(),
//         }))
//     }

//     fn lookup_recursive(
//         &self,
//         path: &str,
//     ) -> filesystem_abstractions::FileSystemResult<Arc<dyn IInode>> {
//         self.should_be_dir()?;

//         let allocated: String;
//         let path = match path::is_path_fully_qualified(path) {
//             true => path,
//             false => {
//                 allocated = path::combine(&self.inner().path(), path)
//                     .ok_or(FileSystemError::InvalidInput)?;
//                 &allocated
//             }
//         };

//         let inode = self
//             .fs
//             .file_builder()
//             .open(path)
//             .map_err(|_| FileSystemError::NotFound)?;

//         Ok(Arc::new(Lwext4Inode {
//             inner: UnsafeCell::new(inode),
//             filename: path::get_filename(path).to_string(),
//             fs: self.fs.clone(),
//         }))
//     }

//     fn metadata(
//         &self,
//     ) -> filesystem_abstractions::FileSystemResult<filesystem_abstractions::InodeMetadata> {
//         let metadata = self
//             .inner()
//             .metadata()
//             .map_err(|_| FileSystemError::InternalError)?;
//         let entry_type = metadata.to_file_type();

//         let children_count = match entry_type {
//             filesystem_abstractions::DirectoryEntryType::Directory => self
//                 .fs
//                 .readdir(self.inner().path())
//                 .map_err(|_| FileSystemError::InternalError)?
//                 .count(),
//             _ => 0,
//         };

//         Ok(filesystem_abstractions::InodeMetadata {
//             size: metadata.size() as usize,
//             filename: &self.filename,
//             entry_type,
//             children_count,
//         })
//     }

//     fn read_dir(
//         &self,
//     ) -> filesystem_abstractions::FileSystemResult<Vec<filesystem_abstractions::DirectoryEntry>>
//     {
//         self.should_be_dir()?;

//         let mut entries = Vec::new();

//         for entry in self
//             .fs
//             .readdir(self.inner().path())
//             .map_err(|_| FileSystemError::InternalError)?
//         {
//             let inode = self
//                 .fs
//                 .file_builder()
//                 .open(entry.path())
//                 .map_err(|_| FileSystemError::InternalError)?;

//             let metadata = inode
//                 .metadata()
//                 .map_err(|_| FileSystemError::FileSystemCorrupted)?;

//             entries.push(filesystem_abstractions::DirectoryEntry {
//                 filename: entry.name().to_string(),
//                 size: metadata.size() as usize,
//                 entry_type: metadata.to_file_type(),
//                 inode: None,
//             });
//         }

//         Ok(entries)
//     }

//     fn mkdir(&self, name: &str) -> filesystem_abstractions::FileSystemResult<Arc<dyn IInode>> {
//         self.should_be_dir()?;

//         let path =
//             path::combine(&self.inner().path(), name).ok_or(FileSystemError::InvalidInput)?;
//         self.fs
//             .create_dir(&path)
//             .map_err(|_| FileSystemError::SpaceNotEnough)?;

//         let inode = self
//             .fs
//             .file_builder()
//             .open(path)
//             .map_err(|_| FileSystemError::InternalError)?;

//         Ok(Arc::new(Lwext4Inode {
//             inner: UnsafeCell::new(inode),
//             filename: name.to_string(),
//             fs: self.fs.clone(),
//         }))
//     }

//     fn touch(&self, name: &str) -> filesystem_abstractions::FileSystemResult<Arc<dyn IInode>> {
//         self.should_be_dir()?;

//         let path =
//             path::combine(&self.inner().path(), name).ok_or(FileSystemError::InvalidInput)?;
//         let inode = self
//             .fs
//             .file_builder()
//             .read(true)
//             .write(true)
//             .create(true)
//             .open(path)
//             .map_err(|_| FileSystemError::SpaceNotEnough)?;

//         Ok(Arc::new(Lwext4Inode {
//             inner: UnsafeCell::new(inode),
//             filename: name.to_string(),
//             fs: self.fs.clone(),
//         }))
//     }

//     fn remove(&self, name: &str) -> filesystem_abstractions::FileSystemResult<()> {
//         self.should_be_dir()?;

//         let path =
//             path::combine(&self.inner().path(), name).ok_or(FileSystemError::InvalidInput)?;

//         self.fs
//             .remove_file(path)
//             .map_err(|_| FileSystemError::InternalError)
//     }

//     fn rmdir(&self, name: &str) -> filesystem_abstractions::FileSystemResult<()> {
//         self.should_be_dir()?;

//         let path =
//             path::combine(&self.inner().path(), name).ok_or(FileSystemError::InvalidInput)?;

//         self.fs
//             .remove_dir(path)
//             .map_err(|_| FileSystemError::InternalError)
//     }

//     fn readat(
//         &self,
//         offset: usize,
//         buffer: &mut [u8],
//     ) -> filesystem_abstractions::FileSystemResult<usize> {
//         self.should_be_file()?;

//         let metadata = self
//             .inner()
//             .metadata()
//             .map_err(|_| FileSystemError::InternalError)?;
//         let file_size = metadata.size() as usize;

//         if offset >= file_size {
//             return Ok(0);
//         }

//         let read_size = core::cmp::min(buffer.len(), file_size - offset);

//         self.inner()
//             .seek(embedded_io::SeekFrom::Start(offset as u64))
//             .map_err(|_| FileSystemError::InternalError)?;

//         self.inner().read(&mut buffer[..read_size]).map_err(|_| {
//             self.inner()
//                 .seek(SeekFrom::Current(-(offset as i64)))
//                 .expect("Failed to seek back");
//             FileSystemError::InternalError
//         })
//     }

//     fn writeat(
//         &self,
//         offset: usize,
//         buffer: &[u8],
//     ) -> filesystem_abstractions::FileSystemResult<usize> {
//         self.should_be_file()?;

//         self.inner()
//             .seek(embedded_io::SeekFrom::Start(offset as u64))
//             .map_err(|_| FileSystemError::InternalError)?;

//         self.inner().write(buffer).map_err(|_| {
//             self.inner()
//                 .seek(SeekFrom::Current(-(offset as i64)))
//                 .expect("Failed to seek back");
//             FileSystemError::InternalError
//         })
//     }

//     fn stat(
//         &self,
//         _stat: &mut filesystem_abstractions::FileStatistics,
//     ) -> filesystem_abstractions::FileSystemResult<()> {
//         todo!()
//     }
// }

// trait IFileType {
//     fn to_file_type(&self) -> filesystem_abstractions::DirectoryEntryType;
// }

// impl IFileType for lwext4_rs::Metadata {
//     fn to_file_type(&self) -> filesystem_abstractions::DirectoryEntryType {
//         if self.is_dir() {
//             filesystem_abstractions::DirectoryEntryType::Directory
//         } else if self.is_file() {
//             filesystem_abstractions::DirectoryEntryType::File
//         } else {
//             panic!("Unsupported file type: {:?}", self.file_type());
//         }
//     }
// }

// use core::ffi::c_char;
// use core::ffi::c_int;
// use core::ffi::c_void;

// type CInt = ::core::ffi::c_int;
// type CChar = u8;
// type CSizeT = usize;

// #[no_mangle]
// #[allow(non_upper_case_globals)]
// static stdout: usize = 0;

// #[no_mangle]
// extern "C" fn fflush(file: *mut c_void) -> c_int {
//     assert!(file.is_null());
//     0
// }

// #[no_mangle]
// unsafe extern "C" fn qsort(
//     base: *mut c_void,
//     nmemb: CSizeT,
//     width: CSizeT,
//     compar: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int>,
// ) {
//     let compar = compar.unwrap();

//     if nmemb <= 1 {
//         return;
//     }

//     let base = base.cast::<u8>();
//     let mut gap = nmemb;

//     loop {
//         gap = next_gap(gap);

//         let mut any_swapped = false;
//         let mut a = base;
//         let mut b = base.add(gap * width);
//         for _ in 0..nmemb - gap {
//             if compar(a.cast(), b.cast()) > 0 {
//                 swap(a, b, width);
//                 any_swapped = true;
//             }
//             a = a.add(width);
//             b = b.add(width);
//         }

//         if gap <= 1 && !any_swapped {
//             break;
//         }
//     }
// }

// fn next_gap(gap: CSizeT) -> CSizeT {
//     let gap = (gap * 10) / 13;

//     if gap == 9 || gap == 10 {
//         11 // apply the "rule of 11"
//     } else if gap <= 1 {
//         1
//     } else {
//         gap
//     }
// }

// unsafe fn swap(a: *mut u8, b: *mut u8, width: CSizeT) {
//     for i in 0..width {
//         core::ptr::swap(a.add(i), b.add(i));
//     }
// }

// #[no_mangle]
// pub unsafe extern "C" fn strcmp(s1: *const CChar, s2: *const CChar) -> CInt {
//     for i in 0.. {
//         let s1_i = s1.offset(i);
//         let s2_i = s2.offset(i);

//         let val = *s1_i as CInt - *s2_i as CInt;
//         if val != 0 || *s1_i == 0 {
//             return val;
//         }
//     }
//     0
// }

// #[no_mangle]
// pub unsafe extern "C" fn strncpy(
//     dest: *mut CChar,
//     src: *const CChar,
//     count: usize,
// ) -> *const CChar {
//     let mut i = 0;
//     while i < count {
//         let c = *src.add(i);
//         *dest.add(i) = c;
//         i += 1;
//         if c == 0 {
//             break;
//         }
//     }
//     for j in i..count {
//         *dest.add(j) = 0;
//     }
//     dest
// }

// #[no_mangle]
// pub unsafe extern "C" fn strcpy(dest: *mut CChar, src: *const CChar) -> *const CChar {
//     let mut i = 0;
//     loop {
//         *dest.offset(i) = *src.offset(i);
//         let c = *dest.offset(i);
//         if c == 0 {
//             break;
//         }
//         i += 1;
//     }
//     dest
// }

// #[no_mangle]
// pub unsafe extern "C" fn strncmp(s1: *const CChar, s2: *const CChar, n: usize) -> CInt {
//     for i in 0..n as isize {
//         let s1_i = s1.offset(i);
//         let s2_i = s2.offset(i);

//         let val = *s1_i as CInt - *s2_i as CInt;
//         if val != 0 || *s1_i == 0 {
//             return val;
//         }
//     }
//     0
// }

// const MAX_ALIGN: usize = 16;

// #[no_mangle]
// pub unsafe extern "C" fn malloc(size: CSizeT) -> *mut u8 {
//     // size + MAX_ALIGN for to store the size of the allocated memory.
//     let layout = alloc::alloc::Layout::from_size_align(size + MAX_ALIGN, MAX_ALIGN).unwrap();
//     let ptr = unsafe { alloc::alloc::alloc(layout) };
//     if ptr.is_null() {
//         return ptr;
//     }
//     unsafe {
//         *(ptr as *mut CSizeT) = size;
//     }
//     unsafe { ptr.add(MAX_ALIGN) }
// }

// #[no_mangle]
// pub unsafe extern "C" fn free(ptr: *mut u8) {
//     if ptr.is_null() {
//         return;
//     }

//     let old_size = unsafe { *(ptr.sub(MAX_ALIGN) as *mut CSizeT) };
//     let layout = alloc::alloc::Layout::from_size_align(old_size + MAX_ALIGN, MAX_ALIGN).unwrap();
//     unsafe { alloc::alloc::dealloc(ptr.sub(MAX_ALIGN), layout) };
// }

// #[no_mangle]
// pub unsafe extern "C" fn calloc(nmemb: CSizeT, size: CSizeT) -> *mut u8 {
//     let total_size = nmemb * size;
//     let layout = alloc::alloc::Layout::from_size_align(total_size + MAX_ALIGN, MAX_ALIGN).unwrap();
//     let ptr = unsafe { alloc::alloc::alloc_zeroed(layout) };
//     if ptr.is_null() {
//         return ptr;
//     }
//     unsafe {
//         *(ptr as *mut CSizeT) = total_size;
//     }
//     unsafe { ptr.add(MAX_ALIGN) }
// }

// #[no_mangle]
// pub unsafe extern "C" fn realloc(ptr: *mut u8, size: CSizeT) -> *mut u8 {
//     if ptr.is_null() {
//         return malloc(size);
//     }
//     let old_size = unsafe { *(ptr.sub(MAX_ALIGN) as *mut CSizeT) };
//     let layout = alloc::alloc::Layout::from_size_align(old_size + MAX_ALIGN, MAX_ALIGN).unwrap();
//     let new_ptr = unsafe { alloc::alloc::realloc(ptr.sub(MAX_ALIGN), layout, size + MAX_ALIGN) };
//     if new_ptr.is_null() {
//         return new_ptr;
//     }
//     unsafe {
//         *(new_ptr as *mut CSizeT) = size;
//     }
//     unsafe { new_ptr.add(MAX_ALIGN) }
// }

// #[no_mangle]
// unsafe extern "C" fn printf(_str: *const c_char) -> c_int {
//     0
// }
