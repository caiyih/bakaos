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
    fn create_stream<'a>(&'a self, cursor: VirtualAddress, keep_buffer: bool) -> MemoryStream<'a>;

    fn create_cross_stream<'a>(
        &'a mut self,
        src: &'a dyn IMMU,
        cursor: VirtualAddress,
        keep_buffer: bool,
    ) -> MemoryStream<'a>;
}

impl IMMUStreamExt for dyn IMMU {
    fn create_stream(&self, cursor: VirtualAddress, keep_buffer: bool) -> MemoryStream<'_> {
        MemoryStream::new(self, cursor, keep_buffer)
    }

    fn create_cross_stream<'a>(
        &'a mut self,
        src: &'a dyn IMMU,
        cursor: VirtualAddress,
        keep_buffer: bool,
    ) -> MemoryStream<'a> {
        MemoryStream::new_cross(self, Some(src), cursor, keep_buffer)
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

enum MmuComposition<'a> {
    Single(&'a dyn IMMU),
    Cross {
        mmu: UnsafeCell<&'a mut dyn IMMU>,
        src: &'a dyn IMMU,
    },
}

macro_rules! impl_stream {
    ($type:tt) => {
        pub struct $type<'a> {
            mmu: MmuComposition<'a>,
            inner: UnsafeCell<MemoryWindow>,
            buffer_keep: Option<Vec<VirtualAddress>>,
        }

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
                    mmu: MmuComposition::Single(mmu),
                    inner: UnsafeCell::new(MemoryWindow {
                        cursor,
                        window: None,
                    }),
                    buffer_keep: if keep_buffer { Some(Vec::new()) } else { None },
                }
            }

            /// Create a new memory stream reader with cross MMU.
            ///
            /// # Arguments
            ///
            /// * `mmu` - The MMU to use.
            /// * `cross` - The cross MMU to use.
            /// * `cursor` - The initial cursor.
            /// * `keep_buffer` - Whether to keep the mapped buffer. if false, the mapped buffer
            ///   will be unmap when the cursor moves outside current page. if true, the mapped buffer
            ///   will be unmap when the stream is dropped.
            pub fn new_cross(
                mmu: &'a mut dyn IMMU,
                cross: Option<&'a dyn IMMU>,
                cursor: VirtualAddress,
                keep_buffer: bool,
            ) -> Self {
                let mmu = match cross {
                    Some(cross) => MmuComposition::Cross {
                        mmu: UnsafeCell::new(mmu),
                        src: cross,
                    },
                    None => MmuComposition::Single(mmu),
                };

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
            #[inline]
            fn source(&self) -> &dyn IMMU {
                match self.mmu {
                    MmuComposition::Single(mmu) => mmu,
                    MmuComposition::Cross { src, .. } => src,
                }
            }

            #[inline]
            fn mmu_map_buffer(
                &self,
                cursor: VirtualAddress,
                len: usize,
            ) -> Result<&[u8], MMUError> {
                match &self.mmu {
                    #[allow(deprecated)]
                    MmuComposition::Single(mmu) => mmu.map_buffer_internal(cursor, len),
                    MmuComposition::Cross { mmu, ref src } => {
                        let mmu = unsafe { mmu.get().as_mut().unwrap() };
                        mmu.map_cross_internal(*src, cursor, len)
                    }
                }
            }

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
                        self.source().unmap_buffer(cursor);
                    }
                } else {
                    self.unmap_current();
                }
            }

            fn unmap_current(&self) {
                if let Some(window) = self.inner_mut().window.take() {
                    self.source().unmap_buffer(window.base);
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
                    let (_pa, flags, size) =
                        self.source().query_virtual(cur).map_err(|e| e.into())?;

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
            #[inline]
            fn inspect_slice_internal<T>(
                &mut self,
                len: usize,
                access: MemoryAccess,
                move_cursor: bool,
            ) -> Result<(NonNull<T>, usize), MMUError> {
                let bytes = len.checked_mul(size_of::<T>()).unwrap();
                let cursor = self.cursor();

                if (cursor.as_usize() % align_of::<T>()) != 0 {
                    return Err(MMUError::MisalignedAddress);
                }

                let slice = match self.check_full_range(cursor, bytes, access)? {
                    WindowCheckResult::Reuse if len == 0 => (NonNull::dangling(), 0),
                    WindowCheckResult::Reuse => {
                        let window = self.inner().window.as_ref().unwrap();

                        let offset = cursor.as_usize() - window.base.as_usize();
                        let ptr = unsafe { window.ptr.add(offset).cast() };

                        (ptr, len)
                    }
                    WindowCheckResult::Remap(access, base, size, overlaps) => {
                        match (overlaps, self.buffer_keep.is_some()) {
                            (true, true) => return Err(MMUError::Borrowed),
                            (true, false) => self.unmap_current(),
                            _ => (),
                        }

                        let size = size.as_usize();

                        #[allow(deprecated)]
                        let s = self.mmu_map_buffer(base, size)?;
                        let ptr = s.as_ptr() as *mut u8;

                        if let Some(buffer_keep) = &mut self.buffer_keep {
                            buffer_keep.push(cursor);
                        } else {
                            self.unmap_current();
                        }

                        let ptr = NonNull::new(ptr).unwrap();

                        self.inner_mut().window = Some(MappedWindow {
                            base,
                            ptr,
                            len: size,
                            access,
                        });

                        // The mapped buffer may be the whole page,
                        // so we need to calculate the offset.
                        let idx = (self.cursor().as_usize() - base.as_usize())
                            / core::mem::size_of::<T>();

                        (unsafe { ptr.cast().add(idx) }, len)
                    }
                };

                if move_cursor {
                    self.inner_mut().cursor += bytes;
                }

                Ok(slice)
            }

            #[inline]
            fn read_slice_internal<T>(
                &mut self,
                len: usize,
                move_cursor: bool,
            ) -> Result<&[T], MMUError> {
                let (ptr, len) =
                    self.inspect_slice_internal(len, MemoryAccess::Read, move_cursor)?;
                Ok(unsafe { core::slice::from_raw_parts(ptr.as_ptr(), len) })
            }

            /// Read a slice of `T` from the stream, and move the cursor.
            ///
            /// # Arguments
            ///
            /// * `len` - The number of `T` to read.
            ///
            /// # Returns
            ///
            /// The full slice of `T` read from the stream.
            pub fn read_slice<T>(&mut self, len: usize) -> Result<&[T], MMUError> {
                self.read_slice_internal(len, true)
            }

            /// Read a slice of `T` from the stream, without touching the cursor.
            ///
            /// # Arguments
            ///
            /// * `len` - The number of `T` to read.
            ///
            /// # Returns
            ///
            /// The full slice of `T` read from the stream.
            pub fn pread_slice<T>(&mut self, len: usize) -> Result<&[T], MMUError> {
                self.read_slice_internal(len, false)
            }

            /// Read a `T` from the stream and move the cursor.
            ///
            /// # Returns
            ///
            /// The reference to `T` read from the stream.
            ///
            /// # Remarks
            ///
            /// Note that the returned references may not be continuously if you call this method in a row.
            #[inline]
            pub fn read<T>(&mut self) -> Result<&T, MMUError> {
                let r = self.read_slice_internal::<T>(1, true)?;
                Ok(unsafe { r.get_unchecked(0) })
            }

            /// Read a `T` from the stream without touching the cursor.
            ///
            /// # Returns
            ///
            /// The reference to `T` read from the stream.
            ///
            /// # Remarks
            ///
            /// Note that the returned reference may not be continuously if you call this method in a row.
            #[inline]
            pub fn pread<T>(&mut self) -> Result<&T, MMUError> {
                let r = self.read_slice_internal::<T>(1, false)?;
                Ok(unsafe { r.get_unchecked(0) })
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
            #[inline]
            fn read_unsized_internal<T>(
                &mut self,
                mut callback: impl FnMut(&T, usize) -> bool,
                move_cursor: bool,
            ) -> Result<&[T], MMUError> {
                let cursor = self.cursor();
                let size = core::mem::size_of::<T>();

                if cursor.as_usize() % core::mem::align_of::<T>() != 0 {
                    return Err(MMUError::MisalignedAddress);
                }

                assert!(size > 0);

                let mut len = 0;

                let mut pending_len = 0usize;

                let mut tmp = MaybeUninit::uninit();
                let pending =
                    unsafe { core::slice::from_raw_parts_mut(tmp.as_mut_ptr() as *mut u8, size) };

                self.source()
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

                // FIXME: checks if there's overlap with existing window

                #[allow(deprecated)]
                let slice_bytes = {
                    let slice = self.mmu_map_buffer(cursor, total_bytes)?;

                    // the mapped slice's lifetime is bound to the page table
                    // Rust compiler thinks we can't mutate the slice as there's a immutable page table reference
                    // This is idiot, same lifetime doesn't mean we are accessing the same value.
                    unsafe { core::slice::from_raw_parts(slice.as_ptr(), slice.len()) }
                };

                if let Some(buffer_keep) = &mut self.buffer_keep {
                    buffer_keep.push(cursor);
                } else {
                    self.unmap_current();
                }

                // prevent mapping leaks
                self.inner_mut().window = Some(MappedWindow {
                    base: cursor,
                    ptr: NonNull::new(slice_bytes.as_ptr() as *mut u8).unwrap(),
                    len: slice_bytes.len(),
                    access: MemoryAccess::Read,
                });

                debug_assert!(slice_bytes.as_ptr() as usize % core::mem::align_of::<T>() == 0);

                let slice =
                    unsafe { core::slice::from_raw_parts(slice_bytes.as_ptr() as *const T, len) };

                if move_cursor {
                    self.inner_mut().cursor += total_bytes;
                }

                Ok(slice)
            }

            /// Read a unsized type from the stream.
            ///
            /// # Arguments
            ///
            /// * `callback` - The callback function to determine whether to continue reading.
            ///
            /// # Returns
            ///
            /// The full slice of `T` read from the stream.
            pub fn read_unsized_slice<T>(
                &mut self,
                callback: impl FnMut(&T, usize) -> bool,
            ) -> Result<&[T], MMUError> {
                self.read_unsized_internal(callback, true)
            }

            /// Read a unsized type from the stream without touching the cursor.
            ///
            /// # Arguments
            ///
            /// * `callback` - The callback function to determine whether to continue reading.
            ///
            /// # Returns
            ///
            /// The full slice of `T` read from the stream.
            pub fn pread_unsized_slice<T>(
                &mut self,
                callback: impl FnMut(&T, usize) -> bool,
            ) -> Result<&[T], MMUError> {
                self.read_unsized_internal(callback, false)
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
    use core::mem::size_of;
    use hermit_sync::SpinMutex;
    use mmu_abstractions::{GenericMappingFlags, MMUError, PageSize};
    use test_utilities::allocation::contiguous::TestFrameAllocator;
    use utilities::InvokeOnDrop;

    use super::*;

    type Alloc = Arc<SpinMutex<dyn IFrameAllocator>>;
    type Mmu = Arc<SpinMutex<dyn IMMU>>;

    fn create_alloc_mmu() -> (Alloc, Mmu) {
        TestFrameAllocator::new_with_mmu(1024 * 1024 * 1024) // 1 GB
    }

    // Helper: create a test memory scene with read/write mapping
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

    // Helper: create a test memory scene with readonly mapping
    fn test_scene_readonly(action: impl FnOnce(Arc<SpinMutex<dyn IMMU>>, VirtualAddress, usize)) {
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
                    GenericMappingFlags::User | GenericMappingFlags::Readable,
                )
                .unwrap();
        }

        action(mmu, base, len)
    }

    #[test]
    fn test_stream_creation() {
        // Test stream creation and cursor initialization
        test_scene(|mmu, base, _len| {
            let mmu = mmu.lock();

            let stream = mmu.create_stream(base, false);
            assert_eq!(stream.cursor(), base);

            let stream_with_keep = mmu.create_stream(base, true);
            assert_eq!(stream_with_keep.cursor(), base);
        });
    }

    #[test]
    fn test_cursor_operations() {
        // Test skip and seek operations for the stream cursor
        test_scene(|mmu, base, _len| {
            let mmu = mmu.lock();

            let mut stream = mmu.create_stream(base, false);
            assert_eq!(stream.cursor(), base);

            let new_cursor = stream.skip(8);
            assert_eq!(new_cursor, base + 8);
            assert_eq!(stream.cursor(), base + 8);

            let seek_cursor = stream.seek(Whence::Set(base + 16));
            assert_eq!(seek_cursor, base + 16);
            assert_eq!(stream.cursor(), base + 16);

            let seek_cursor = stream.seek(Whence::Offset(-4));
            assert_eq!(seek_cursor, base + 12);
            assert_eq!(stream.cursor(), base + 12);

            let seek_cursor = stream.seek(Whence::Offset(8));
            assert_eq!(seek_cursor, base + 20);
            assert_eq!(stream.cursor(), base + 20);
        });
    }

    #[test]
    fn test_basic_read() {
        // Test basic read and pread for single value and slice
        test_scene(|mmu, base, _len| {
            let mmu = mmu.lock();

            mmu.export::<i32>(base, 42).unwrap();

            let mut stream = mmu.create_stream(base, false);

            assert_eq!(stream.cursor(), base);

            assert_eq!(*stream.pread::<i32>().unwrap(), 42);
            assert_eq!(stream.cursor(), base);

            assert_eq!(*stream.read::<i32>().unwrap(), 42);
            assert_eq!(stream.cursor(), base + 4);

            let next_val = *stream.pread::<i32>().unwrap();
            let next_val_moved = *stream.read::<i32>().unwrap();

            assert_eq!(next_val, next_val_moved);

            mmu.export::<[i32; 4]>(base, [42, 24, -42, -24]).unwrap();
            assert_eq!(mmu.import::<[i32; 4]>(base).unwrap(), [42, 24, -42, -24]);

            stream.sync(); // We've accessed the memory without this MemoryStream

            stream.seek(Whence::Set(base));
            let result = stream.pread_slice::<i32>(4).unwrap();

            assert_eq!(result, [42, 24, -42, -24]);

            assert_eq!(stream.read_slice::<i32>(4).unwrap(), [42, 24, -42, -24]);
        });
    }

    #[test]
    fn test_read_different_types() {
        test_scene(|mmu, base, _len| {
            let mmu = mmu.lock();

            // Arrange
            mmu.export::<u8>(base, 0xAB).unwrap();
            mmu.export::<u16>(base + 4, 0x1234).unwrap();
            mmu.export::<u32>(base + 8, 0x12345678).unwrap();
            mmu.export::<u64>(base + 16, 0x123456789ABCDEF0).unwrap();

            let mut stream = mmu.create_stream(base, false);

            assert_eq!(*stream.read::<u8>().unwrap(), 0xAB);

            stream.seek(Whence::Set(base + 4));
            assert_eq!(*stream.read::<u16>().unwrap(), 0x1234);

            stream.seek(Whence::Set(base + 8));
            assert_eq!(*stream.read::<u32>().unwrap(), 0x12345678);

            stream.seek(Whence::Set(base + 16));
            assert_eq!(*stream.read::<u64>().unwrap(), 0x123456789ABCDEF0);
        });
    }

    #[test]
    fn test_read_slice_various_sizes() {
        test_scene(|mmu, base, _len| {
            let mmu = mmu.lock();

            // Arrange
            let test_data: [i32; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
            mmu.export::<[i32; 8]>(base, test_data).unwrap();

            let mut stream = mmu.create_stream(base, false);

            assert_eq!(stream.read_slice::<i32>(1).unwrap(), [1]);

            assert_eq!(stream.read_slice::<i32>(2).unwrap(), [2, 3]);
            assert_eq!(stream.read_slice::<i32>(3).unwrap(), [4, 5, 6]);
            assert_eq!(stream.read_slice::<i32>(2).unwrap(), [7, 8]);
        });
    }

    #[test]
    fn test_read_unsized() {
        // Test reading unsized data (null-terminated and terminator-based)
        test_scene(|mmu, base, _len| {
            let mmu = mmu.lock();

            let test_string = b"Hello\0World\0Test\0";
            mmu.write_bytes(base, test_string).unwrap();

            let mut stream = mmu.create_stream(base, false);

            let strings = stream
                .read_unsized_slice::<u8>(|&byte, _| byte != 0)
                .unwrap();
            assert_eq!(strings, b"Hello");

            stream.skip(1); // Skip the null terminator

            let strings = stream
                .read_unsized_slice::<u8>(|&byte, _| byte != 0)
                .unwrap();
            assert_eq!(strings, b"World");

            stream.skip(1);

            let strings = stream
                .read_unsized_slice::<u8>(|&byte, _| byte != 0)
                .unwrap();
            assert_eq!(strings, b"Test");
        });
    }

    #[test]
    fn test_read_unsized_with_limit() {
        test_scene(|mmu, base, _len| {
            let mmu = mmu.lock();

            // Arrange
            let test_data: [i32; 5] = [1, 2, 3, 4, 5];
            mmu.export::<[i32; 5]>(base, test_data).unwrap();

            let mut stream = mmu.create_stream(base, false);

            let mut count = 0;
            let result = stream
                .read_unsized_slice::<i32>(|&_val, _| {
                    count += 1;
                    count <= 3 // Read the first 3 elements
                })
                .unwrap();

            assert_eq!(result, [1, 2, 3]);
            assert_eq!(count, 4);
        });
    }

    #[test]
    fn test_cross_mmu_basic() {
        let (alloc1, mmu1) = create_alloc_mmu();
        let (alloc2, mmu2) = create_alloc_mmu();

        let frames1 = alloc1.lock().alloc_contiguous(5).unwrap();
        let frames1 = InvokeOnDrop::transform(frames1, |f| alloc1.lock().dealloc_range(f));
        let frames2 = alloc2.lock().alloc_contiguous(5).unwrap();
        let frames2 = InvokeOnDrop::transform(frames2, |f| alloc2.lock().dealloc_range(f));

        let len1 = frames1.end.as_usize() - frames1.start.as_usize();
        let len2 = frames2.end.as_usize() - frames2.start.as_usize();

        let page_size1 = len1 / 5;
        let page_size2 = len2 / 5;
        let base1 = VirtualAddress::from_usize(0x10000);
        let base2 = VirtualAddress::from_usize(0x20000);

        for i in 0..5 {
            mmu1.lock()
                .map_single(
                    base1 + i * page_size1,
                    frames1.start + i * page_size1,
                    PageSize::from(page_size1),
                    GenericMappingFlags::User
                        | GenericMappingFlags::Readable
                        | GenericMappingFlags::Writable,
                )
                .unwrap();
        }

        for i in 0..5 {
            mmu2.lock()
                .map_single(
                    base2 + i * page_size2,
                    frames2.start + i * page_size2,
                    PageSize::from(page_size2),
                    GenericMappingFlags::User
                        | GenericMappingFlags::Readable
                        | GenericMappingFlags::Writable,
                )
                .unwrap();
        }

        mmu1.lock().export::<i32>(base1, 42).unwrap();

        let mmu1_ref = mmu1.lock();
        let mut mmu2_guard = mmu2.lock();
        let mut stream = mmu2_guard.create_cross_stream(&*mmu1_ref, base1, false);

        assert_eq!(*stream.read::<i32>().unwrap(), 42);
    }

    #[test]
    fn test_misaligned_address() {
        // Test reading from a misaligned address
        test_scene(|mmu, base, _len| {
            let mmu = mmu.lock();

            let mut stream = mmu.create_stream(base + 1, false);

            let result = stream.read::<i32>();
            assert!(matches!(result, Err(MMUError::MisalignedAddress)));
        });
    }

    #[test]
    fn test_read_only_memory() {
        test_scene_readonly(|mmu, base, _len| {
            let mmu = mmu.lock();

            let result = mmu.export::<i32>(base, 42);
            assert!(result.is_err()); // Should fail due to read-only mapping

            let mut stream = mmu.create_stream(base, false);

            let _val = stream.read::<i32>().unwrap();
        });
    }

    #[test]
    fn test_invalid_address() {
        // Test reading from an invalid address
        test_scene(|mmu, base, _len| {
            let mmu = mmu.lock();

            let mut stream = mmu.create_stream(base, false);
            stream.seek(Whence::Set(VirtualAddress::from_usize(0x10000000)));

            let result = stream.read::<i32>();
            assert!(matches!(result, Err(MMUError::InvalidAddress)));
        });
    }

    #[test]
    fn test_buffer_keep_functionality() {
        test_scene(|mmu, base, _len| {
            let mmu = mmu.lock();

            // Arrange
            let test_data: [i32; 4] = [1, 2, 3, 4];
            mmu.export::<[i32; 4]>(base, test_data).unwrap();

            let mut stream = mmu.create_stream(base, true);

            let slice1 = stream.read_slice::<i32>(2).unwrap();
            assert_eq!(slice1, [1, 2]);

            stream.seek(Whence::Set(base + 8));
            let slice2 = stream.read_slice::<i32>(2).unwrap();
            assert_eq!(slice2, [3, 4]);

            stream.sync();
        });
    }

    #[test]
    fn test_buffer_keep_vs_no_keep() {
        test_scene(|mmu, base, _len| {
            let mmu = mmu.lock();

            let test_data: [i32; 4] = [1, 2, 3, 4];
            mmu.export::<[i32; 4]>(base, test_data).unwrap();

            // test with keep_buffer = false
            {
                let mut stream = mmu.create_stream(base, false);
                let _slice1 = stream.read_slice::<i32>(2).unwrap();

                stream.seek(Whence::Set(base + 8));
                let _slice2 = stream.read_slice::<i32>(2).unwrap();
            }

            // test with keep_buffer = true
            {
                let mut stream = mmu.create_stream(base, true);
                let _slice1 = stream.read_slice::<i32>(2).unwrap();

                // Move to a different position, the buffer should be preserved
                stream.seek(Whence::Set(base + 8));
                // Since the buffer overlaps, this may fail, which is expected
                let result2 = stream.read_slice::<i32>(2);

                if result2.is_err() {
                    // If a Borrowed error occurs, this is the expected behavior
                    assert!(matches!(result2, Err(MMUError::Borrowed)));
                } else {
                    // If no error occurs, validate the data correctness
                    let slice = result2.unwrap();
                    assert_eq!(slice.len(), 2);
                }
            }
        });
    }

    #[test]
    fn test_window_reuse() {
        test_scene(|mmu, base, _len| {
            let mmu = mmu.lock();

            let test_data: [i32; 4] = [1, 2, 3, 4];
            mmu.export::<[i32; 4]>(base, test_data).unwrap();

            let mut stream = mmu.create_stream(base, false);

            let slice1 = stream.pread_slice::<i32>(2).unwrap();
            assert_eq!(slice1, [1, 2]);

            let slice2 = stream.pread_slice::<i32>(2).unwrap();
            assert_eq!(slice2, [1, 2]);

            stream.seek(Whence::Offset(4));
            let slice3 = stream.pread_slice::<i32>(2).unwrap();
            assert_eq!(slice3, [2, 3]);
        });
    }

    #[test]
    fn test_window_remap() {
        test_scene(|mmu, base, _len| {
            let mmu = mmu.lock();

            let test_data: [i32; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
            mmu.export::<[i32; 8]>(base, test_data).unwrap();

            let mut stream = mmu.create_stream(base, false);

            let slice1 = stream.read_slice::<i32>(4).unwrap();
            assert_eq!(slice1, [1, 2, 3, 4]);

            stream.seek(Whence::Set(base + 16));
            let slice2 = stream.read_slice::<i32>(4).unwrap();
            assert_eq!(slice2, [5, 6, 7, 8]);
        });
    }

    #[test]
    fn test_empty_read() {
        // Test reading zero elements returns empty slice
        test_scene(|mmu, base, _len| {
            let mmu = mmu.lock();
            let mut stream = mmu.create_stream(base, false);

            let slice = stream.read_slice::<i32>(0).unwrap();
            assert_eq!(slice.len(), 0);
        });
    }

    #[test]
    fn test_large_read() {
        test_scene(|mmu, base, len| {
            let mmu = mmu.lock();

            let data_size = len / 4;
            let mut test_data = alloc::vec![0i32; data_size];
            for i in 0..data_size {
                test_data[i] = i as i32;
            }

            mmu.write_bytes(base, unsafe {
                core::slice::from_raw_parts(
                    test_data.as_ptr() as *const u8,
                    test_data.len() * size_of::<i32>(),
                )
            })
            .unwrap();

            let mut stream = mmu.create_stream(base, false);

            let slice = stream.read_slice::<i32>(data_size).unwrap();
            assert_eq!(slice.len(), data_size);
            for i in 0..data_size {
                assert_eq!(slice[i], i as i32);
            }
        });
    }

    #[test]
    fn test_seek_boundaries() {
        test_scene(|mmu, base, len| {
            let mmu = mmu.lock();
            let mut stream = mmu.create_stream(base, false);

            // test seek to the end
            let end_addr = base + len;
            stream.seek(Whence::Set(end_addr - 4));
            assert_eq!(stream.cursor(), end_addr - 4);

            // test reading the last element
            let result = stream.read::<i32>();
            assert!(result.is_ok());

            let result = stream.read::<i32>();
            assert!(result.is_err());

            // try read invalid address
            stream.seek(Whence::Set(end_addr));
            let result = stream.read::<i32>();
            assert!(matches!(result, Err(MMUError::InvalidAddress)));
        });
    }

    #[test]
    fn test_consecutive_reads() {
        test_scene(|mmu, base, _len| {
            let mmu = mmu.lock();

            let _test_data: [i32; 100] = {
                let mut data = [0i32; 100];
                for i in 0..100 {
                    data[i] = i as i32;
                }
                data
            };

            for i in 0..100 {
                mmu.export::<i32>(base + i * 4, i as i32).unwrap();
            }

            // Assert that the data is correctly written
            for i in 0..10 {
                let val = mmu.import::<i32>(base + i * 4).unwrap();
                assert_eq!(val, i as i32);
            }

            let mut stream = mmu.create_stream(base, false);

            for i in 0..100 {
                let val = *stream.read::<i32>().unwrap();
                assert_eq!(val, i as i32);
            }
        });
    }

    #[test]
    fn test_mixed_read_operations() {
        test_scene(|mmu, base, _len| {
            let mmu = mmu.lock();

            let test_data: [i32; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
            mmu.export::<[i32; 10]>(base, test_data).unwrap();

            let mut stream = mmu.create_stream(base, false);

            assert_eq!(*stream.read::<i32>().unwrap(), 1);
            assert_eq!(stream.read_slice::<i32>(3).unwrap(), [2, 3, 4]);
            assert_eq!(*stream.read::<i32>().unwrap(), 5);
            assert_eq!(stream.read_slice::<i32>(2).unwrap(), [6, 7]);
            assert_eq!(*stream.read::<i32>().unwrap(), 8);
            assert_eq!(stream.read_slice::<i32>(2).unwrap(), [9, 10]);
        });
    }
}
