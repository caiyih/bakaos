use alloc::{sync::Arc, vec::Vec};
use core::{fmt::Debug, slice};
use page_table::GenericMappingFlags;

use abstractions::IUsizeAlias;
use address::{
    IConvertablePhysicalAddress, IPageNum, IToPageNum, PhysicalPageNum, VirtualAddress,
    VirtualPageNum, VirtualPageNumRange,
};
use allocation::TrackedFrame;
use bitflags::bitflags;
use filesystem_abstractions::{
    DirectoryEntryType, DirectoryTreeNode, FileCacheAccessor, FileDescriptor, FileMetadata,
    ICacheableFile, IFile,
};

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct MemoryMapProt: u32 {
        const NONE = 0;
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const EXECUTE = 1 << 2;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct MemoryMapFlags: u32 {
        const ANONYMOUS = 0x00;
        const SHARED = 0x01;
        const PRIVATE = 0x02;
    }
}

impl MemoryMapFlags {
    pub fn is_anonymous(&self) -> bool {
        (self.bits() & 0b11) == 0b00
    }
}

#[derive(Debug, Clone)]
pub struct MemoryMapRecord {
    pub prot: MemoryMapProt,
    pub offset: usize,
    pub length: usize,
    pub page_area: VirtualPageNumRange,
}

pub struct TaskMemoryMap {
    mmap_top: VirtualPageNum,
    records: Vec<MemoryMapRecord>,
    mapped_files: Vec<(Arc<MemoryMappedFile>, Arc<FileCacheAccessor>)>,
}

impl Default for TaskMemoryMap {
    fn default() -> Self {
        TaskMemoryMap {
            mmap_top: VirtualAddress::from_usize(
                // TODO: Find out a better range of mmap area
                // 0x100_000_0000
                constants::PAGE_SIZE * constants::PAGE_SIZE * 4096,
            )
            .to_floor_page_num(),
            records: Vec::new(),
            mapped_files: Vec::new(),
        }
    }
}

impl Clone for TaskMemoryMap {
    fn clone(&self) -> Self {
        Self {
            mmap_top: self.mmap_top,
            records: self.records.clone(),
            mapped_files: self.mapped_files.clone(),
        }
    }
}

impl TaskMemoryMap {
    pub fn records(&self) -> &[MemoryMapRecord] {
        &self.records
    }

    pub fn allocate_records(
        &mut self,
        length: usize,
        _preferred_addr: Option<VirtualAddress>,
    ) -> Option<VirtualPageNumRange> {
        let page_count = length.div_ceil(constants::PAGE_SIZE);

        // let start_page = match preferred_addr {
        //     None => self.mmap_top - length.div_ceil(constants::PAGE_SIZE) - 1,
        //     Some(addr) => {
        //         let start_page = addr.to_floor_page_num();

        //
        //         let area = VirtualPageNumRange::from_start_count(start_page, page_count);

        //         // check if overlapping
        //         for record in self.records.iter() {
        //             if record.page_area.contains(area.start())
        //                 || record.page_area.contains(area.end() - 1)
        //                 || record.page_area.contained_by(&area)
        //             {
        //                 return None;
        //             }
        //         }

        //         start_page
        //     }
        // };

        let start_page = self.mmap_top - page_count - 1;
        let area = VirtualPageNumRange::from_start_count(start_page, page_count);

        self.records.push(MemoryMapRecord {
            prot: MemoryMapProt::all(),
            offset: 0,
            length,
            page_area: area,
        });

        self.mmap_top = start_page;

        Some(area)
    }

    pub fn mmap(
        &mut self,
        fd: Option<&Arc<FileDescriptor>>,
        flags: MemoryMapFlags,
        prot: MemoryMapProt,
        offset: usize,
        length: usize,
        mut register_page: impl FnMut(VirtualPageNum, PhysicalPageNum, GenericMappingFlags),
    ) -> Option<VirtualAddress> {
        let mapped_file_idx = self.get_create_mapped_file(fd, flags, length)?;

        let mapped_file = &self.mapped_files[mapped_file_idx].0;

        let start_frame_idx = offset / constants::PAGE_SIZE;
        let page_count = length.div_ceil(constants::PAGE_SIZE);

        let mut start_page = self.mmap_top - page_count - 1;

        for record in self.records.iter() {
            start_page = core::cmp::min(start_page, record.page_area.start() - page_count - 1);
        }

        for i in 0..page_count {
            let ppn = mapped_file.frames[start_frame_idx + i].ppn();
            let vpn = start_page + i;

            let mut permissions = GenericMappingFlags::User;

            // TODO: Can be optimized with bit ops
            if prot.contains(MemoryMapProt::READ) {
                permissions |= GenericMappingFlags::Readable;
            }

            if prot.contains(MemoryMapProt::WRITE) {
                permissions |= GenericMappingFlags::Writable;
            }

            if prot.contains(MemoryMapProt::EXECUTE) {
                permissions |= GenericMappingFlags::Executable;
            }

            register_page(vpn, ppn, permissions);
        }

        self.mmap_top = start_page - 1;

        self.records.push(MemoryMapRecord {
            prot,
            offset,
            length,
            page_area: VirtualPageNumRange::from_start_count(start_page, page_count),
        });

        Some(start_page.at_offset_of_start(offset % constants::PAGE_SIZE))
    }

    pub fn munmap(
        &mut self,
        addr: VirtualAddress,
        length: usize,
        mut revoke_registration: impl FnMut(VirtualPageNum),
    ) -> bool {
        if length == 0 {
            return true;
        }

        // Find target record
        let target_record_idx = self
            .records
            .iter()
            .position(|record| record.page_area.contains(addr.to_floor_page_num()));

        if target_record_idx.is_none() {
            return false;
        }

        let target_record_idx = target_record_idx.unwrap();
        let target_record = &self.records[target_record_idx];

        let length = core::cmp::min(length, target_record.length);

        let start_page_to_be_unmapped = addr.to_floor_page_num(); // Linux requires any page that containing the address to be unmapped
        let end_page_to_be_unmapped = (addr + length).to_ceil_page_num(); // End page is exclusive

        for vpn in
            VirtualPageNumRange::from_start_end(start_page_to_be_unmapped, end_page_to_be_unmapped)
                .iter()
        {
            revoke_registration(vpn);
        }

        // Consider the case where we have to split the record
        if end_page_to_be_unmapped != target_record.page_area.end() {
            let page_area = VirtualPageNumRange::from_start_end(
                end_page_to_be_unmapped,
                target_record.page_area.end(),
            );

            debug_assert!(page_area.page_count() != 0);
            let in_page_offset =
                (target_record.length + target_record.offset) % constants::PAGE_SIZE;

            let offset = (target_record.offset + length + constants::PAGE_SIZE - 1)
                & !(constants::PAGE_SIZE - 1);

            // Align the offset to the page boundary
            debug_assert!(offset % constants::PAGE_SIZE == 0);

            let length = in_page_offset + (page_area.page_count() - 1) * constants::PAGE_SIZE;

            let new_record = MemoryMapRecord {
                prot: target_record.prot,
                offset,
                length,
                page_area,
            };

            self.records.push(new_record);
        }

        // Update offset and length of the target record
        let target_record = &mut self.records[target_record_idx];
        target_record.page_area = VirtualPageNumRange::from_start_end(
            target_record.page_area.start(),
            start_page_to_be_unmapped,
        );

        target_record.length = target_record.page_area.page_count() * constants::PAGE_SIZE
            - (target_record.offset % constants::PAGE_SIZE);

        true
    }

    fn get_create_mapped_file(
        &mut self,
        fd: Option<&Arc<FileDescriptor>>,
        flags: MemoryMapFlags,
        length: usize,
    ) -> Option<usize> {
        if fd.is_none() || flags.is_anonymous() {
            self.mapped_files
                .push(MemoryMappedFile::new_anonymous(length));

            return Some(self.mapped_files.len() - 1);
        }

        let fd = fd.unwrap();

        for (idx, (_, accessor)) in self.mapped_files.iter().enumerate() {
            if accessor.table_idx() == fd.fd_idx() {
                return Some(idx);
            }
        }

        let created = MemoryMappedFile::new_named(fd.clone(), flags)?;
        self.mapped_files.push(created);
        Some(self.mapped_files.len() - 1)
    }
}

impl TaskMemoryMap {
    pub fn end_mappings(&mut self) {
        for (file, accessor) in self.mapped_files.iter() {
            match &file.map_type {
                MemoryMapInner::Named { named_map_type } => match &named_map_type {
                    NamedMemoryMapInner::Shared { associated_file } => {
                        let original_file = associated_file;
                        let inode = original_file.inode().unwrap();
                        let mut cache_entry = unsafe { accessor.access_cache_entry() };

                        // Write all the content back to the inode
                        for (idx, frame) in file.frames.iter().enumerate() {
                            let ppn = frame.ppn();
                            let offset = idx * constants::PAGE_SIZE;
                            let length = core::cmp::min(constants::PAGE_SIZE, file.length - offset);

                            let ptr =
                                unsafe { ppn.start_addr().to_high_virtual().as_mut_ptr::<u8>() };
                            let slice = unsafe { core::slice::from_raw_parts_mut(ptr, length) };

                            // ignore the result
                            let _ = inode.writeat(offset, slice);
                        }

                        cache_entry.cahce = original_file.clone();
                    }
                    NamedMemoryMapInner::Private {
                        associated_file_handle: _,
                    } => (), // Same as below
                },
                MemoryMapInner::Anonymous => (), // Cleaned up as accessors are dropped
                                                 // Since only *this* process can access the anonymous memory map,
                                                 // and when this method is called, the process is exiting,
                                                 // we don't need to clean up the anonymous memory map
            }
        }
    }
}

enum NamedMemoryMapInner {
    Shared {
        associated_file: Arc<dyn IFile>,
    },
    Private {
        associated_file_handle: Arc<FileCacheAccessor>,
    },
}

enum MemoryMapInner {
    Named { named_map_type: NamedMemoryMapInner },
    Anonymous,
}

// When open a mmap file, the original file will be replaced by this MemoryMappedFile
struct MemoryMappedFile {
    length: usize,
    frames: Vec<TrackedFrame>,
    map_type: MemoryMapInner,
}

impl MemoryMappedFile {
    pub fn new_named(
        fd: Arc<FileDescriptor>,
        flags: MemoryMapFlags,
    ) -> Option<(Arc<Self>, Arc<FileCacheAccessor>)> {
        if flags.is_anonymous() {
            return None;
        }

        let inode = fd.access().metadata()?.inode();
        let file_size = inode.metadata().size;

        let initialized_frames = Self::allocate_named(inode)?;

        let file = if flags.contains(MemoryMapFlags::SHARED) {
            Arc::new(MemoryMappedFile {
                length: file_size,
                frames: initialized_frames,
                map_type: MemoryMapInner::Named {
                    named_map_type: NamedMemoryMapInner::Shared {
                        associated_file: fd.access(),
                    },
                },
            })
        } else if flags.contains(MemoryMapFlags::PRIVATE) {
            Arc::new(MemoryMappedFile {
                length: file_size,
                frames: initialized_frames,
                map_type: MemoryMapInner::Named {
                    named_map_type: NamedMemoryMapInner::Private {
                        associated_file_handle: fd.file_handle().clone_non_inherited_arc(),
                    },
                },
            })
        } else {
            return None;
        };

        {
            // Replace the original file with the memory mapped file
            let mut cache_entry = unsafe { fd.access_cache_entry() };
            cache_entry.cahce = file.clone();

            // drop lock to file
        }

        Some((file, fd.file_handle().clone_non_inherited_arc()))
    }

    // Since we have to replace the whole file, we have to copy all the content from the inode to the frames
    fn allocate_named(inode: Arc<DirectoryTreeNode>) -> Option<Vec<TrackedFrame>> {
        let metadata = inode.metadata();

        if metadata.entry_type != DirectoryEntryType::File {
            return None;
        }

        let size = metadata.size;

        let frame_count = size.div_ceil(constants::PAGE_SIZE);

        let frames = allocation::alloc_frames(frame_count)
            .expect("Failed to allocate that much frames for named memory map");

        // Copy the content from the inode to the frames
        for (idx, frames) in frames.iter().enumerate() {
            let ppn = frames.ppn();

            let offset = idx * constants::PAGE_SIZE;
            let length = core::cmp::min(constants::PAGE_SIZE, size - offset);

            let ptr = unsafe { ppn.start_addr().to_high_virtual().as_mut_ptr::<u8>() };
            let slice = unsafe { core::slice::from_raw_parts_mut(ptr, length) };

            inode.readat(offset, slice).ok()?;
        }

        Some(frames)
    }

    pub fn new_anonymous(length: usize) -> (Arc<Self>, Arc<FileCacheAccessor>) {
        let file = Arc::new(MemoryMappedFile::allocate_anonymous(length));
        let ifile: Arc<dyn IFile> = file.clone();

        let accessor = ifile.cache_as_arc_accessor();

        (file, accessor)
    }

    fn allocate_anonymous(length: usize) -> Self {
        let frame_count = length.div_ceil(constants::PAGE_SIZE);
        let frames = allocation::alloc_frames(frame_count)
            .expect("Failed to allocate that much frames for anonymous memory map");
        MemoryMappedFile {
            frames,
            map_type: MemoryMapInner::Anonymous,
            length: frame_count * constants::PAGE_SIZE,
        }
    }
}

impl IFile for MemoryMappedFile {
    fn metadata(&self) -> Option<Arc<FileMetadata>> {
        match &self.map_type {
            MemoryMapInner::Named {
                named_map_type: NamedMemoryMapInner::Shared { associated_file },
            } => associated_file.metadata(),
            MemoryMapInner::Named {
                named_map_type:
                    NamedMemoryMapInner::Private {
                        associated_file_handle,
                    },
            } => associated_file_handle.access().metadata(),
            MemoryMapInner::Anonymous => None,
        }
    }

    fn can_read(&self) -> bool {
        true
    }

    fn can_write(&self) -> bool {
        true
    }

    fn read_avaliable(&self) -> bool {
        true
    }

    fn write_avaliable(&self) -> bool {
        true
    }

    fn inode(&self) -> Option<Arc<DirectoryTreeNode>> {
        // Prevent access to the inode before we completely restore the file
        None
    }

    fn is_dir(&self) -> bool {
        false
    }

    fn write(&self, buf: &[u8]) -> usize {
        let metadata = match self.metadata() {
            Some(metadata) => metadata,
            None => return 0,
        };

        let size = self.length;
        let offset = metadata.offset();

        // Write to frames
        let mut current_frame_idx = offset / constants::PAGE_SIZE;
        let mut current_offset = offset;

        if current_frame_idx >= self.frames.len() {
            return 0;
        }

        loop {
            let in_page_len = core::cmp::min(constants::PAGE_SIZE, size - current_offset);
            let in_page_len = core::cmp::min(in_page_len, buf.len() - (current_offset - offset));

            let ppn = self.frames[current_frame_idx].ppn();
            let ptr = unsafe { ppn.start_addr().to_high_virtual().as_mut_ptr::<u8>() };

            let bytes_read = current_offset - offset;

            let src_slice = &buf[bytes_read..bytes_read + in_page_len];
            let dst_slice = unsafe { core::slice::from_raw_parts_mut(ptr, in_page_len) };

            dst_slice.copy_from_slice(src_slice);

            current_offset += in_page_len;
            current_frame_idx += 1;

            if current_offset >= size || bytes_read + in_page_len >= buf.len() {
                break;
            }
        }

        current_offset - offset
    }

    fn read(&self, buf: &mut [u8]) -> usize {
        let metadata = match self.metadata() {
            Some(metadata) => metadata,
            None => return 0,
        };

        let size = self.length;
        let offset = metadata.offset();

        // Read from frames
        let mut current_frame_idx = offset / constants::PAGE_SIZE;
        let mut current_offset = offset;

        if current_frame_idx >= self.frames.len() {
            return 0;
        }

        loop {
            let in_page_len = core::cmp::min(constants::PAGE_SIZE, size - current_offset);
            let in_page_len = core::cmp::min(in_page_len, buf.len() - (current_offset - offset));

            let ppn = self.frames[current_frame_idx].ppn();
            let ptr = ppn.start_addr().to_high_virtual().as_ptr::<u8>();

            let src_slice = unsafe { slice::from_raw_parts(ptr, in_page_len) };
            let dst_start = current_offset - offset;

            let dst_slice = &mut buf[dst_start..dst_start + in_page_len];

            dst_slice.copy_from_slice(src_slice);

            current_offset += in_page_len;
            current_frame_idx += 1;

            if current_offset >= size || (current_offset - offset) >= buf.len() {
                break;
            }
        }

        current_offset - offset
    }
}
