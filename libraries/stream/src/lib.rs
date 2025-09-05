#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use core::{cell::UnsafeCell, mem::MaybeUninit, ptr::NonNull};

use abstractions::operations::IUsizeAlias;
use address::{VirtualAddress, VirtualAddressRange};
use alloc::vec::Vec;

use mmu_abstractions::{GenericMappingFlags, MMUError, PageSize, IMMU};

pub trait IMMUStreamExt {
    fn create_stream(&self, cursor: VirtualAddress, keep_buffer: bool) -> MemoryStream<'_>;
}

impl IMMUStreamExt for dyn IMMU {
    fn create_stream(&self, cursor: VirtualAddress, keep_buffer: bool) -> MemoryStream<'_> {
        MemoryStream::new(self, cursor, keep_buffer)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Whence {
    Set(VirtualAddress),
    Offset(isize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MemoryAccess {
    None = 0,
    Read = 1,
    Write = 2,
}

enum WindowCheckResult {
    Reuse,
    Remap(MemoryAccess, VirtualAddress, PageSize, bool),
}

struct MemoryWindow {
    cursor: VirtualAddress,
    window: Option<MappedWindow>,
}

impl MemoryWindow {
    pub fn skip(&mut self, len: usize) -> VirtualAddress {
        self.seek(Whence::Offset(len as isize))
    }

    pub fn seek(&mut self, whence: Whence) -> VirtualAddress {
        let target = match whence {
            Whence::Set(offset) => offset,
            Whence::Offset(off) => VirtualAddress::from_usize(
                (self.cursor.as_usize() as isize).wrapping_add(off) as usize,
            ),
        };

        self.cursor = target;

        target
    }
}

struct MappedWindow {
    base: VirtualAddress,
    ptr: NonNull<u8>,
    len: usize,
    access: MemoryAccess,
}

pub struct MemoryStream<'a> {
    mmu: &'a dyn IMMU,
    inner: UnsafeCell<MemoryWindow>,
    buffer_keep: Option<Vec<VirtualAddress>>,
}

macro_rules! impl_stream {
    ($type:tt) => {
        impl<'a> $type<'a> {
            /// Create a new memory stream reader.
            ///
            /// # Arguments
            ///
            /// * `mmu` - The MMU to use.
            /// * `cursor` - The initial cursor.
            /// * `keep_buffer` - Whether to keep the mapped buffer. if false, the mapped buffer
            ///   will be unmap when the cursor moves outside current page. if true, the mapped buffer
            ///   will be unmap when the stream is dropped.
            pub fn new(mmu: &'a dyn IMMU, cursor: VirtualAddress, keep_buffer: bool) -> Self {
                Self {
                    mmu,
                    inner: UnsafeCell::new(MemoryWindow {
                        cursor,
                        window: None,
                    }),
                    buffer_keep: if keep_buffer { Some(Vec::new()) } else { None },
                }
            }
        }

        impl $type<'_> {
            #[inline(always)]
            fn inner(&self) -> &MemoryWindow {
                unsafe { self.inner.get().as_ref().unwrap() }
            }

            #[inline(always)]
            #[allow(clippy::mut_from_ref)]
            fn inner_mut(&self) -> &mut MemoryWindow {
                unsafe { self.inner.get().as_mut().unwrap() }
            }

            /// Skip `len` bytes in the stream.
            ///
            /// # Arguments
            ///
            /// * `len` - The number of bytes to skip.
            #[inline(always)]
            pub fn skip(&self, len: usize) -> VirtualAddress {
                self.inner_mut().skip(len)
            }

            /// Seek to the given offset.
            ///
            /// # Arguments
            ///
            /// * `whence` - The offset to seek to.
            #[inline(always)]
            pub fn seek(&mut self, whence: Whence) -> VirtualAddress {
                self.inner_mut().seek(whence)
            }

            /// Get the current cursor
            #[inline(always)]
            pub fn cursor(&self) -> VirtualAddress {
                self.inner().cursor
            }

            /// Sync mapped buffers, will unmap all existing buffers
            ///
            /// # Remarks
            ///
            /// If you accessed the memory without this MemoryStream,
            /// Call this method to sync states.
            pub fn sync(&mut self) {
                if let Some(mut buffer_keep) = core::mem::take(&mut self.buffer_keep) {
                    while let Some(cursor) = buffer_keep.pop() {
                        self.mmu.unmap_buffer(cursor);
                    }
                } else {
                    self.unmap_current();
                }
            }

            fn unmap_current(&self) {
                if let Some(window) = self.inner_mut().window.take() {
                    self.mmu.unmap_buffer(window.base);
                }
            }

            fn check_full_range(
                &self,
                start: VirtualAddress,
                len: usize,
                required: MemoryAccess,
            ) -> Result<WindowCheckResult, MMUError> {
                let mut access = MemoryAccess::Write;

                if len == 0 {
                    // TODO: should we still check permission for empty range?
                    return Ok(WindowCheckResult::Reuse);
                }

                let mut overlaps = false;

                if let Some(window) = self.inner().window.as_ref() {
                    let window_range = VirtualAddressRange::from_start_len(window.base, window.len);
                    let range = VirtualAddressRange::from_start_len(start, len);

                    // contains
                    if window_range.contains_range(range) {
                        return ensure_access(start, window.access, required)
                            .map(|_| WindowCheckResult::Reuse);
                    }

                    if window_range.intersects(range) {
                        overlaps = true;
                    }
                }

                let end = start + len;
                let mut cur = start;

                let mut base = None;
                let mut total_size = PageSize::from(0);

                while cur < end {
                    let (_pa, flags, size) = self.mmu.query_virtual(cur).map_err(|e| e.into())?;

                    access = access.min(flags_to_access(flags));

                    ensure_access(cur, access, required)?;

                    let sz = size.as_usize();

                    if base.is_none() {
                        base = Some(VirtualAddress::from_usize(cur.as_usize() / sz * sz));
                    }

                    total_size = PageSize::from(total_size.as_usize() + sz);

                    let cur_u = cur.as_usize();
                    let off_in_page = cur_u % sz;
                    let step = core::cmp::min(sz - off_in_page, end.as_usize() - cur_u);

                    cur += step;
                }

                Ok(WindowCheckResult::Remap(
                    access,
                    base.unwrap(),
                    total_size,
                    overlaps,
                ))
            }
        }

        // Read view
        impl $type<'_> {
            /// Read a slice of `T` from the stream.
            ///
            /// # Arguments
            ///
            /// * `len` - The number of `T` to read.
            /// * `move_cursor` - Whether to move the cursor after reading.
            pub fn read_slice<T>(
                &mut self,
                len: usize,
                move_cursor: bool,
            ) -> Result<&[T], MMUError> {
                let bytes = len.checked_mul(size_of::<T>()).unwrap();
                let cursor = self.inner().cursor;

                if (self.inner().cursor.as_usize() % align_of::<T>()) != 0 {
                    return Err(MMUError::MisalignedAddress);
                }

                let slice = match self.check_full_range(cursor, bytes, MemoryAccess::Read)? {
                    WindowCheckResult::Reuse => {
                        let window = self.inner().window.as_ref().unwrap();

                        let offset = cursor.as_usize() - window.base.as_usize();
                        let ptr = unsafe { window.ptr.as_ptr().add(offset) as *const T };

                        unsafe { core::slice::from_raw_parts(ptr, len) }
                    }
                    WindowCheckResult::Remap(access, base, size, overlaps) => {
                        if overlaps {
                            self.unmap_current();
                        }

                        #[allow(deprecated)]
                        let s = self.mmu.map_buffer_internal(base, size.as_usize())?;
                        let t_ptr = s.as_ptr() as *const T;

                        if let Some(buffer_keep) = &mut self.buffer_keep {
                            buffer_keep.push(cursor);
                        } else {
                            self.unmap_current();
                        }

                        self.inner_mut().window = Some(MappedWindow {
                            base: self.inner().cursor,
                            ptr: NonNull::new(s.as_ptr() as *mut u8).unwrap(),
                            len: s.len(),
                            access,
                        });

                        unsafe { core::slice::from_raw_parts(t_ptr, len) }
                    }
                };

                if move_cursor {
                    self.inner_mut().cursor += bytes;
                }

                Ok(slice)
            }

            /// Read a `T` from the stream.
            ///
            /// # Arguments
            ///
            /// * `move_cursor` - Whether to move the cursor after reading.
            ///
            /// # Remarks
            ///
            /// Note that the returned references may not be continuously if you call this method in a row.
            pub fn read<T>(&mut self, move_cursor: bool) -> Result<&T, MMUError> {
                let r = self.read_slice::<T>(1, move_cursor)?;
                Ok(&r[0])
            }

            /// Read a unsized type from the stream.
            ///
            /// # Arguments
            ///
            /// * `callback` - The callback function to determine whether to continue reading.
            /// * `move_cursor` - Whether to move the cursor after reading.
            ///
            /// # Returns
            ///
            /// The full slice of `T` read from the stream.
            pub fn read_unsized<T>(
                &mut self,
                mut callback: impl FnMut(&T, usize) -> bool,
                move_cursor: bool,
            ) -> Result<&[T], MMUError> {
                let cursor = self.inner().cursor;
                let size = core::mem::size_of::<T>();

                assert!(size > 0);

                let mut len = 0;

                let mut pending_len = 0usize;

                let mut tmp = MaybeUninit::uninit();
                let pending =
                    unsafe { core::slice::from_raw_parts_mut(tmp.as_mut_ptr() as *mut u8, size) };

                self.mmu
                    .inspect_framed_internal(cursor, usize::MAX, &mut |bytes, _| {
                        let mut i = 0;

                        while i < bytes.len() {
                            let need = size - pending_len;
                            let take = core::cmp::min(need, bytes.len() - i);

                            pending[pending_len..pending_len + take]
                                .copy_from_slice(&bytes[i..i + take]);

                            pending_len += take;
                            i += take;

                            if pending_len == size {
                                if !callback(unsafe { tmp.assume_init_ref() }, len) {
                                    return false; // stop scan
                                }

                                len += 1;
                                pending_len = 0;
                            }
                        }

                        true
                    })?;

                let total_bytes = len * size;

                #[allow(deprecated)]
                let slice_bytes = self.mmu.map_buffer_internal(cursor, total_bytes)?;

                if let Some(buffer_keep) = &mut self.buffer_keep {
                    buffer_keep.push(cursor);
                } else {
                    self.unmap_current();
                }

                if (slice_bytes.as_ptr() as usize) % core::mem::align_of::<T>() != 0 {
                    return Err(MMUError::MisalignedAddress);
                }

                let slice =
                    unsafe { core::slice::from_raw_parts(slice_bytes.as_ptr() as *const T, len) };

                if move_cursor {
                    self.inner_mut().cursor += total_bytes;
                }

                Ok(slice)
            }
        }

        impl Drop for $type<'_> {
            fn drop(&mut self) {
                self.sync();
            }
        }
    };
}

impl_stream!(MemoryStream);

#[inline(always)]
const fn flags_to_access(flags: GenericMappingFlags) -> MemoryAccess {
    let mut access = MemoryAccess::None;

    if flags.contains(GenericMappingFlags::Readable) {
        if flags.contains(GenericMappingFlags::Writable) {
            access = MemoryAccess::Write;
        } else {
            access = MemoryAccess::Read;
        }
    }

    access
}

#[inline(always)]
fn ensure_access(
    vaddr: VirtualAddress,
    existing: MemoryAccess,
    required: MemoryAccess,
) -> Result<(), MMUError> {
    if required <= existing {
        return Ok(());
    }

    if required == MemoryAccess::Write {
        return Err(MMUError::PageNotWritable { vaddr });
    }

    if required == MemoryAccess::Read {
        return Err(MMUError::PageNotReadable { vaddr });
    }

    unreachable!();
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;
    use allocation_abstractions::IFrameAllocator;
    use hermit_sync::SpinMutex;
    use mmu_abstractions::PageSize;
    use test_utilities::allocation::contiguous::TestFrameAllocator;
    use utilities::InvokeOnDrop;

    use super::*;

    fn create_alloc_mmu() -> (
        Arc<SpinMutex<dyn IFrameAllocator>>,
        Arc<SpinMutex<dyn IMMU>>,
    ) {
        TestFrameAllocator::new_with_mmu(1 * 1024 * 1024 * 1024)
    }

    fn test_scene(action: impl FnOnce(Arc<SpinMutex<dyn IMMU>>, VirtualAddress, usize)) {
        let (alloc, mmu) = create_alloc_mmu();

        let frames = alloc.lock().alloc_contiguous(10).unwrap();
        let frames = InvokeOnDrop::transform(frames, |f| alloc.lock().dealloc_range(f));

        let len = frames.end.as_usize() - frames.start.as_usize();

        let page_size = len / 10;
        let base = VirtualAddress::from_usize(0x10000);
        for i in 0..10 {
            mmu.lock()
                .map_single(
                    base + i * page_size,
                    frames.start + i * page_size,
                    PageSize::from(page_size),
                    GenericMappingFlags::User
                        | GenericMappingFlags::Readable
                        | GenericMappingFlags::Writable,
                )
                .unwrap();
        }

        action(mmu, base, len)
    }

    #[test]
    fn test_basic_read() {
        test_scene(|mmu, base, _len| {
            let mmu = mmu.lock();

            mmu.export::<i32>(base, 42).unwrap();

            let mut stream = mmu.create_stream(base, false);

            assert_eq!(stream.cursor(), base);

            assert_eq!(*stream.read::<i32>(false).unwrap(), 42);
            assert_eq!(stream.cursor(), base);

            assert_eq!(*stream.read::<i32>(true).unwrap(), 42);
            assert_eq!(stream.cursor(), base + 4);

            assert_ne!(*stream.read::<i32>(false).unwrap(), 42);
            assert_ne!(*stream.read::<i32>(true).unwrap(), 42);

            mmu.export::<[i32; 4]>(base, [42, 24, -42, -24]).unwrap();
            assert_eq!(mmu.import::<[i32; 4]>(base).unwrap(), [42, 24, -42, -24]);

            // we've written the memory with the `export` method
            stream.sync();

            stream.seek(Whence::Set(base));

            assert_eq!(
                stream.read_slice::<i32>(4, false).unwrap(),
                [42, 24, -42, -24]
            );

            assert_eq!(
                stream.read_slice::<i32>(4, true).unwrap(),
                [42, 24, -42, -24]
            );
        });
    }
}
