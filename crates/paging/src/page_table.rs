use alloc::{collections::BTreeMap, sync::Arc, vec, vec::Vec};
use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    slice,
};
use hermit_sync::SpinMutex;
use log::{debug, trace};

use abstractions::{impl_arith_ops, impl_bitwise_ops, impl_bitwise_ops_with, IUsizeAlias};
use address::{
    IAddress, IAlignableAddress, IPageNum, IToPageNum, PhysicalAddress, PhysicalPageNum,
    VirtualAddress, VirtualAddressRange, VirtualPageNum, VirtualPageNumRange,
};
use allocation::frame::TrackedFrame;
use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord)]
    pub struct PageTableEntryFlags : usize {
        const Valid = 1 << 0;
        const Readable = 1 << 1;
        const Writable = 1 << 2;
        const Executable = 1 << 3;
        const User = 1 << 4;
        const Global = 1 << 5;
        const Accessed = 1 << 6;
        const Dirty = 1 << 7;
        // Reserved 1 << 8
        const _Reserved8 = 1 << 8;
    }
}

impl abstractions::IUsizeAlias for PageTableEntryFlags {
    fn as_usize(&self) -> usize {
        self.bits()
    }

    fn from_usize(value: usize) -> Self {
        Self::from_bits_retain(value)
    }
}

impl_bitwise_ops_with!(PageTableEntryFlags, PageTableEntry);

pub const PAGE_TABLE_ENTRY_FLAGS_MASK: usize = 0x1FF;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageTableEntry(usize);

impl PageTableEntry {
    pub const fn new(ppn: PhysicalPageNum, flags: PageTableEntryFlags) -> Self {
        PageTableEntry((ppn.0 << 10) | flags.bits())
    }

    pub fn flags(&self) -> PageTableEntryFlags {
        PageTableEntryFlags::from_bits_truncate(self.0)
    }

    pub fn ppn(&self) -> PhysicalPageNum {
        PhysicalPageNum::from_usize((self.0 >> 10) & ((1usize << 44) - 1))
    }

    pub fn empty() -> Self {
        PageTableEntry(0)
    }
}

impl abstractions::IUsizeAlias for PageTableEntry {
    fn as_usize(&self) -> usize {
        self.0
    }

    fn from_usize(value: usize) -> Self {
        PageTableEntry(value)
    }
}

impl_arith_ops!(PageTableEntry);
impl_bitwise_ops!(PageTableEntry);
impl_bitwise_ops_with!(PageTableEntry, PageTableEntryFlags);

impl PageTableEntry {
    pub fn is_valid(&self) -> bool {
        self.flags().contains(PageTableEntryFlags::Valid)
    }

    pub fn is_readable(&self) -> bool {
        self.flags().contains(PageTableEntryFlags::Readable)
    }

    pub fn is_writable(&self) -> bool {
        self.flags().contains(PageTableEntryFlags::Writable)
    }

    pub fn is_executable(&self) -> bool {
        self.flags().contains(PageTableEntryFlags::Executable)
    }

    pub fn is_user(&self) -> bool {
        self.flags().contains(PageTableEntryFlags::User)
    }

    pub fn is_accessed(&self) -> bool {
        self.flags().contains(PageTableEntryFlags::Accessed)
    }

    pub fn is_dirty(&self) -> bool {
        self.flags().contains(PageTableEntryFlags::Dirty)
    }
}

pub trait IRawPageTable: IPageNum {
    /// Create a slice of page table entries from the physical page number
    /// The returned slice is in the virtual address space, so we can use it directly
    /// # Safety
    /// The caller must ensure that the physical page number is valid
    unsafe fn as_entries(&self) -> &'static mut [PageTableEntry] {
        let page_num = self.as_usize();
        let ptr = (page_num << 12) | constants::VIRT_ADDR_OFFSET;

        unsafe {
            core::slice::from_raw_parts_mut(
                ptr as *mut PageTableEntry,
                constants::PAGE_SIZE / core::mem::size_of::<PageTableEntry>(), // 512
            )
        }
    }
}

impl IRawPageTable for PhysicalPageNum {}

static mut KERNEL_PAGE_TABLE: Option<PageTable> = None;

pub fn get_kernel_page_table() -> &'static PageTable {
    unsafe {
        #[allow(static_mut_refs)]
        KERNEL_PAGE_TABLE
            .as_ref()
            .expect("Kernel page table is not initialized")
    }
}

impl Drop for PageTable {
    fn drop(&mut self) {
        match self.tracker {
            Some(_) => {
                let activated = self.is_activated();
                trace!(
                    "Droping owned page table: {}, activated: {}",
                    self.root_ppn(),
                    activated
                );

                if activated {
                    trace!("Activating kernel page table for the activated page table is being dropped");
                    unsafe {
                        // Lazy switch to kernel page table mechanism implementation:
                        // When we are executing a task, or process in what are used to, we are using the page table of that task.
                        // But when the task/process has ended its life cycle, the task control block is dropped, and the page table is also dropped.
                        // If the page table's frame is rewritten, the page table will be invalid, and the kernel will panic.
                        // So we have to switch to another valid page table before dropping the current page table.
                        // The most reliable way is to switch to the kernel page table, which is always valid.
                        // We only do this when we are dropping a page table that actually owns the page table frames and when the page table is activated.
                        get_kernel_page_table().activate();
                    }
                }
            }
            None => debug!("Droping borrowed page table: {}", self.root_ppn()),
        }
    }
}

impl PageTable {
    pub fn borrow_from_root(root_ppn: PhysicalPageNum) -> PageTable {
        PageTable {
            root: root_ppn,
            tracker: None,
        }
    }

    pub fn borrow_current() -> PageTable {
        #[cfg(not(target_arch = "riscv64"))]
        panic!("Unsupported architecture");

        #[cfg(target_arch = "riscv64")]
        {
            let satp: usize;
            unsafe {
                core::arch::asm!("csrr {}, satp", out(reg) satp);
            }

            let root_ppn = PhysicalPageNum::from_usize(satp & 0x7FFFFFFFFFFFFFFF);

            PageTable::borrow_from_root(root_ppn)
        }
    }
}

pub fn init_kernel_page_table(kernel_table: PageTable) {
    unsafe {
        KERNEL_PAGE_TABLE = Some(kernel_table);
    }
}

struct ModifiablePageTable {
    table_frames: Vec<TrackedFrame>,
    /// # WARNING
    /// Remember to call `restore_temporary_modified_pages` before returning to the user space
    temporary_modified_pages: BTreeMap<VirtualPageNum, TemporaryModifiedPage>,
}

unsafe impl Sync for ModifiablePageTable {}
unsafe impl Send for ModifiablePageTable {}

// A page table is a tree of page table entries
// Represent a SV39 page table and exposes many useful methods
// to work with the memory space of the current page table
pub struct PageTable {
    root: PhysicalPageNum,
    tracker: Option<Arc<SpinMutex<ModifiablePageTable>>>,
}

// Consturctor and Properties
impl PageTable {
    #[allow(clippy::vec_init_then_push)] // see comments below
    pub fn allocate() -> Self {
        let frame =
            allocation::alloc_frame().expect("Failed to allocate a frame for the root page table");

        let root = frame.ppn();

        debug!("Allocating page table at: {}", root);

        let tracker = ModifiablePageTable {
            table_frames: vec![frame],
            temporary_modified_pages: BTreeMap::new(),
        };

        Self {
            root,
            tracker: Some(Arc::new(SpinMutex::new(tracker))),
        }
    }

    pub fn root_ppn(&self) -> PhysicalPageNum {
        self.root
    }

    pub fn satp(&self) -> usize {
        self.root.as_usize() | (8 << 60)
    }

    /// Writes the token of this page table to the satp register
    /// # Safety
    /// This method is unsafe because it writes to the satp register
    /// If the page table is not valid or not mapped the higher half address,
    /// it will cause a page fault
    pub unsafe fn activate(&self) {
        #[cfg(not(target_arch = "riscv64"))]
        panic!("Unsupported architecture");

        #[cfg(target_arch = "riscv64")]
        {
            // This avoids losing CPU cache if the page table is already activated
            // But the scheduler still have to do something else to avoid calling this method multiple times
            if self.is_activated() {
                return;
            }

            trace!("Activating page table: {}", self.root_ppn());

            let satp = self.satp();

            unsafe {
                core::arch::asm!("csrw satp, {}", in(reg) satp);
            }

            self.flush_tlb();
        }
    }

    pub fn is_activated(&self) -> bool {
        #[cfg(not(target_arch = "riscv64"))]
        panic!("Unsupported architecture");

        #[cfg(target_arch = "riscv64")]
        {
            let current_satp: usize;

            unsafe {
                core::arch::asm!("csrr {}, satp", out(reg) current_satp);
            }

            current_satp == self.satp()
        }
    }
}

// Methods
impl PageTable {
    pub fn map_single(
        &mut self,
        vpn: VirtualPageNum,
        ppn: PhysicalPageNum,
        flags: PageTableEntryFlags,
    ) {
        let entry = self.get_create_entry_of(vpn);
        assert!(!entry.is_valid(), "The entry is already mapped.");
        *entry = PageTableEntry::new(
            ppn,
            flags
                | PageTableEntryFlags::Valid
                | PageTableEntryFlags::Accessed
                | PageTableEntryFlags::Dirty,
        );
    }

    pub fn unmap_single(&mut self, vpn: VirtualPageNum) {
        let entry = self
            .get_entry_of(vpn)
            .expect("Attempted to unmap an unmapped page");
        *entry = PageTableEntry::empty();
    }
}

// internal methods
impl PageTable {
    pub fn get_entry_of(&self, vpn: VirtualPageNum) -> Option<&mut PageTableEntry> {
        let indices = vpn.page_table_indices();
        let mut table_ppn = self.root_ppn();

        let mut res = Option::None;

        for (level, index) in indices.iter().enumerate() {
            let table = unsafe { table_ppn.as_entries() };
            let entry = &mut table[*index];

            if level == 2 {
                res = Some(entry);
                break;
            }

            if !entry.is_valid() {
                return None;
            }

            table_ppn = entry.ppn();
        }

        res
    }

    fn get_create_entry_of(&mut self, vpn: VirtualPageNum) -> &mut PageTableEntry {
        debug_assert!(self.tracker.is_some(), "Page table is not modifiable");

        let indices = vpn.page_table_indices();
        let mut table_ppn = self.root_ppn();

        let tracker = unsafe { &mut self.tracker.as_mut().unwrap_unchecked().lock() };
        for (level, index) in indices.iter().enumerate() {
            let table = unsafe { table_ppn.as_entries() };
            let entry = &mut table[*index];

            if level == 2 {
                return entry;
            }

            if !entry.is_valid() {
                let frame = allocation::alloc_frame()
                    .expect("Failed to allocate a frame for the page table");
                *entry = PageTableEntry::new(frame.ppn(), PageTableEntryFlags::Valid);
                tracker.table_frames.push(frame);
            }

            table_ppn = entry.ppn();
        }

        unreachable!()
    }
}

impl PageTable {
    // Get physical address of a virtual address in current page table
    // And returns the high half mapped virtual address
    pub fn as_high_half(&self, addr: VirtualAddress) -> Option<(PhysicalAddress, VirtualAddress)> {
        // Fast path for already mapped high half address
        if addr.as_usize() & constants::VIRT_ADDR_OFFSET == constants::VIRT_ADDR_OFFSET {
            return Some((addr.to_low_physical(), addr));
        }

        let vpn = addr.to_floor_page_num();
        let offset = addr.in_page_offset();

        let ppn = self.get_entry_of(vpn)?.ppn();

        let pa = ppn.at_offset_of_start::<PhysicalAddress>(offset);

        Some((pa, pa.to_high_virtual()))
    }

    /// Get physical address of a virtual address in current page table
    /// And returns the high half mapped virtual address
    /// # Safety
    /// Must check that if the reference is across page boundary.
    /// You have to handle it manually if it's neither continuous nor in the same page
    /// You may want to use `CopyToSpace` methods
    pub unsafe fn as_high_half_ptr<T>(&self, ptr: *const T) -> Option<(PhysicalAddress, *mut T)> {
        let addr = VirtualAddress::from_ptr(ptr);
        self.as_high_half(addr)
            .map(|(pa, _)| (pa, unsafe { pa.to_high_virtual().as_mut_ptr::<T>() }))
    }
}

impl PageTable {
    /// Copy data to the memory space. The data must be in the current memory space.
    pub fn activated_copy_data_to_other(
        &self,
        dst: &PageTable,
        offset: VirtualAddress,
        data: &[u8],
    ) -> usize {
        debug_assert!(self.is_activated());

        let mut copied = 0;

        while copied < data.len() {
            let va = dst
                .as_high_half(offset + copied)
                .expect("Virtual address is not mapped")
                .1;
            let end = va.page_up();

            let chunk = usize::min(data.len() - copied, (end - va).as_usize());

            unsafe {
                self.activated_copy_data_to(va, &data[copied..copied + chunk]);
            }

            copied += chunk;
        }

        copied
    }

    /// Copy data to the memory space. The data must be in the current memory space.
    /// # Safety
    /// This function is unsafe because it directly writes to the memory space.
    /// You have to make sure that the memory space is active, which means that
    /// the satp register is set to the page table of this memory space.
    pub unsafe fn activated_copy_data_to(&self, offset: VirtualAddress, data: &[u8]) -> usize {
        debug_assert!(self.is_activated());

        let slice = unsafe { slice::from_raw_parts_mut(offset.as_mut_ptr::<u8>(), data.len()) };

        match (
            self.guard_slice(data).with_read(),
            self.guard_slice(slice).with_write(),
        ) {
            (Some(from_guard), Some(mut to_guard)) => {
                to_guard.as_mut().copy_from_slice(&from_guard);
                data.len()
            }
            _ => 0,
        }
    }

    /// Copy data across memory spaces. There's no limitation on the source and destination memory spaces.
    /// But it's also the slowest way to copy data. as we have to split the data into many chunks.
    /// Still pretty fast if the data and offset is page-aligned.
    /// # Safety
    /// This method uses the high half address to access the data.
    /// The high half address is already mapped by the frame allocator, and has all the permissions.
    /// We are reading from src and writing to dst at the Physical Page, with the high half address.
    pub fn copy_across_spaces(
        src: &PageTable,
        data: &[u8],
        dst: &PageTable,
        offset: VirtualAddress,
    ) -> usize {
        let mut copied = 0;
        let src_offet = VirtualAddress::from_ptr(data.as_ptr());

        while copied < data.len() {
            match (
                src.as_high_half(src_offet + copied),
                dst.as_high_half(offset + copied),
            ) {
                (Some((_, src_va)), Some((_, dst_va))) => {
                    let src_len = constants::PAGE_SIZE - src_va.in_page_offset();
                    let dst_len = constants::PAGE_SIZE - dst_va.in_page_offset();

                    let chunk = usize::min(usize::min(src_len, dst_len), data.len() - copied);

                    let src_slice = unsafe { slice::from_raw_parts(src_va.as_ptr::<u8>(), chunk) };

                    let dst_slice =
                        unsafe { slice::from_raw_parts_mut(dst_va.as_mut_ptr::<u8>(), chunk) };

                    // Don't have to use guard because we are copying using high half address,
                    // which is already mapped by the frame allocator, and has all the permissions
                    dst_slice.copy_from_slice(src_slice);

                    copied += chunk;
                }
                _ => return copied,
            }
        }

        copied
    }

    pub fn activated_copy_val_to<T>(&self, offset: VirtualAddress, data: &T) -> usize {
        debug_assert!(self.is_activated());

        let data = unsafe {
            slice::from_raw_parts(data as *const _ as *const u8, core::mem::size_of::<T>())
        };

        unsafe { self.activated_copy_data_to(offset, data) }
    }

    pub fn copy_slice_to<T>(&self, offset: VirtualAddress, data: &[T]) -> usize {
        debug_assert!(self.is_activated());

        let data = unsafe {
            slice::from_raw_parts(data as *const _ as *const u8, core::mem::size_of::<T>())
        };

        unsafe { self.activated_copy_data_to(offset, data) }
    }

    pub fn activated_copy_val_to_other<T>(
        &self,
        offset: VirtualAddress,
        data_space: &PageTable,
        data: &T,
    ) -> usize {
        let data = unsafe {
            slice::from_raw_parts(data as *const _ as *const u8, core::mem::size_of::<T>())
        };

        // fast path for destnation spaces that are activated
        if data_space.is_activated() {
            return data_space.activated_copy_data_to_other(self, offset, data);
        }

        PageTable::copy_across_spaces(self, data, data_space, offset)
    }

    pub fn copy_slice_to_other<T>(
        &self,
        offset: VirtualAddress,
        data_space: &PageTable,
        data: &[T],
    ) -> usize {
        let data = unsafe {
            slice::from_raw_parts(data.as_ptr() as *const u8, core::mem::size_of_val(data))
        };

        // fast path for destnation spaces that are activated
        if data_space.is_activated() {
            return data_space.activated_copy_data_to_other(self, offset, data);
        }

        PageTable::copy_across_spaces(self, data, data_space, offset)
    }
}

impl PageTable {
    pub fn temporary_switch_to(&self, other: &PageTable) -> TemporarySwitchGuard {
        unsafe {
            other.activate();
        }

        TemporarySwitchGuard { page_table: self }
    }
}

pub struct TemporarySwitchGuard<'a> {
    page_table: &'a PageTable,
}

impl Drop for TemporarySwitchGuard<'_> {
    fn drop(&mut self) {
        unsafe {
            self.page_table.activate();
        }
    }
}

pub struct TemporaryModificationGuard<'a> {
    page_table: &'a PageTable,
}

impl Drop for TemporaryModificationGuard<'_> {
    #[allow(invalid_reference_casting)]
    fn drop(&mut self) {
        self.page_table.restore_temporary_modified_pages();
    }
}

impl PageTable {
    pub fn begin_temporary_modification(&self) -> TemporaryModificationGuard {
        TemporaryModificationGuard { page_table: self }
    }

    // You don't have to call this method manually as long as you created the guard
    pub fn restore_temporary_modified_pages(&self) {
        match &self.tracker {
            None => debug!("Ignoring for borrowed page table"),
            Some(tracker) => {
                let modified_pages = &mut tracker.lock().temporary_modified_pages;
                // Prevent flushing tlb if there is no modification
                if modified_pages.is_empty() {
                    return;
                }

                for modification in modified_pages.iter() {
                    debug!(
                        "Restoring page: {} to {:?}, current: {:?}",
                        modification.0, modification.1.previous, modification.1.now
                    );
                    let entry = self.get_entry_of(*modification.0).unwrap();
                    *entry = PageTableEntry::new(entry.ppn(), modification.1.previous);
                }

                modified_pages.clear();

                self.flush_tlb();
            }
        }
    }

    /// If you want to modify the page table persistently,
    /// you should use the following methods instead of modifying the page table directly
    #[allow(clippy::option_map_unit_fn)]
    pub fn persistent_add(&mut self, vpn: VirtualPageNum, flags: PageTableEntryFlags) {
        debug_assert!(self.tracker.is_some(), "Page table is not modifiable");

        let entry = self.get_create_entry_of(vpn);
        *entry |= flags;

        let tracker = unsafe { &mut self.tracker.as_mut().unwrap_unchecked() };
        // Update the temporary modified pages
        tracker
            .lock()
            .temporary_modified_pages
            .entry(vpn)
            .and_modify(|e| e.now |= flags); // not add if not exist

        self.flush_tlb();
    }

    /// If you want to modify the page table persistently,
    /// you should use the following methods instead of modifying the page table directly
    #[allow(clippy::option_map_unit_fn)]
    pub fn persistent_remove(&mut self, vpn: VirtualPageNum, flags: PageTableEntryFlags) {
        debug_assert!(self.tracker.is_some(), "Page table is not modifiable");

        let entry = self.get_entry_of(vpn).unwrap();
        *entry &= !flags;

        let tracker = unsafe { self.tracker.as_mut().unwrap_unchecked() };
        // Update the temporary modified pages
        tracker
            .lock()
            .temporary_modified_pages
            .entry(vpn)
            .and_modify(|e| e.now &= !flags); // not add if not exist

        self.flush_tlb();
    }

    pub fn flush_tlb(&self) {
        #[cfg(not(target_arch = "riscv64"))]
        panic!("Unsupported architecture");

        #[cfg(target_arch = "riscv64")]
        unsafe {
            core::arch::asm!("sfence.vma")
        }
    }
}

// Permission Guard
impl PageTable {
    /// Checks if the given slice has the specified flags and adds any missing flags with fluent creation design pattern
    /// See `guard_vpn_range` for more information
    pub fn guard_slice<'a, TValue>(
        &'a self,
        slice: &[TValue],
    ) -> PageGuardBuilder<'a, &'static [TValue]> {
        let address_range = VirtualAddressRange::from_slice(slice);
        let vpn_range = VirtualPageNumRange::from_start_end(
            address_range.start().to_floor_page_num(),
            address_range.end().to_ceil_page_num(),
        );

        let mut guard = self.guard_vpn_range(vpn_range);
        guard.ptr = slice.as_ptr() as usize;
        guard.len = slice.len();

        unsafe { core::mem::transmute::<_, PageGuardBuilder<'a, &[TValue]>>(guard) }
    }

    /// Checks if the given range of virtual pages has the specified flags and adds any missing flags with fluent creation design pattern
    /// The virtual address is in the virtual address space of `this` page table. And all `guard` methods are based on this.
    /// So all returned guards with auto dereference are valid only in the virtual address space of `this` page table.
    /// # Example
    /// ```ignore
    /// let guard = page_table.guard_ref(some_pointer_from_user_space)
    ///     .must_have(PageTableEntryFlags::Executable) // We to make sure the user space can read the data. And Valid flag is automatically added
    ///     .with(PageTableEntryFlags::Readable) // We need to write the data, so add with this method
    ///     .unwrap(); // If `must_have` not satisfied, it will return None
    ///
    /// let some_value = guard; // Now we can read the value
    ///
    /// page_table.restore_temporary_modified_pages(); // Don't forget to restore the temporary modified pages
    ///                                                // Usually you only have to call this method when you are returning to the user space
    /// ```
    ///
    /// Multilpe guards can be created with
    /// ```ignore
    /// let guard1 = page_table.guard_ref(some_slice_from_user_space)
    ///     .with(PageTableEntryFlags::Readable) // This is added lazily, if already readable, it will not be added
    ///     .unwrap();
    ///
    /// let mut guard2 = page_table.guard_ref(another_slice_from_user_space)
    ///     .with(PageTableEntryFlags::Writable) // If the two guards are overlapping, the flags will be merged, resulting both readable and writable
    ///     .unwrap();
    ///
    /// guard1.as_mut().copy_from_slice(guard2); // Now we can copy the data from one slice to another
    ///                                          // Note that some guard methods like guard_ref and guard_slice implement `AsMut` and `Deref` traits
    ///                                          // So you can use them as a normal slice or reference
    /// ```
    ///
    /// Mutable reference can be obtained with `AsMut` trait, as shown in the example above
    /// ```ignore
    /// let mut guard = page_table.guard_ref(some_pointer_from_user_space)
    ///     .must_have(PageTableEntryFlags::Readable)
    ///     .with(PageTableEntryFlags::Writable) // We need to write the data, so add with this method
    ///     .unwrap();
    ///
    /// let p = guard.as_mut(); // Now we can write the value
    /// *p = 42;
    /// ```
    pub fn guard_vpn_range(&self, vpn_range: VirtualPageNumRange) -> PageGuardBuilder<()> {
        PageGuardBuilder {
            page_table: self,
            vpn_range,
            ptr: vpn_range.start().start_addr::<VirtualAddress>().as_usize(),
            len: 0,
            _marker: PhantomData,
        }
    }

    /// Guard permission of a single virtual page number. See `guard_vpn_range` for more information
    pub fn guard_vpn(&self, vpn: VirtualPageNum) -> PageGuardBuilder<()> {
        self.guard_vpn_range(VirtualPageNumRange::from_start_end(vpn, vpn + 1))
    }

    /// Guard permission of a reference. See `guard_vpn_range` for more information
    pub fn guard_ref<'a, T>(&'a self, value: &T) -> PageGuardBuilder<'a, &'static T> {
        let address = VirtualAddress::from_usize(value as *const T as usize);
        let mut guard = self.guard_vpn_range(VirtualPageNumRange::from_start_end(
            address.to_floor_page_num(),
            (address + core::mem::size_of::<T>()).to_ceil_page_num(),
        ));

        guard.ptr = value as *const T as usize;
        guard.len = 1; // Not needed actually
        unsafe { core::mem::transmute::<_, PageGuardBuilder<'a, &T>>(guard) }
    }

    pub fn guard_ptr<'a, T>(&'a self, value: *const T) -> PageGuardBuilder<'a, &'static T> {
        let address = VirtualAddress::from_usize(value as usize);
        let mut guard = self.guard_vpn_range(VirtualPageNumRange::from_start_end(
            address.to_floor_page_num(),
            (address + core::mem::size_of::<T>()).to_ceil_page_num(),
        ));

        guard.ptr = value as usize;
        guard.len = 1; // Not needed actually
        unsafe { core::mem::transmute::<_, PageGuardBuilder<'a, &T>>(guard) }
    }

    pub fn guard_cstr(
        &self,
        ptr: *const u8,
        max_len: usize,
    ) -> UnsizedSlicePageGuardBuilder<'_, &'static [u8]> {
        UnsizedSlicePageGuardBuilder {
            page_table: self,
            vpn_start: VirtualAddress::from_ptr(ptr).to_floor_page_num(),
            ptr: ptr as usize,
            max_len,
            terminator_predicate: Some(|c, idx| c[idx] == 0),
            exclusive_end: true,
            _marker: PhantomData,
        }
    }

    pub fn guard_unsized_cstr_array(
        &self,
        ptr: *const *const u8,
        max_len: usize,
    ) -> UnsizedSlicePageGuardBuilder<'_, &'static [*const u8]> {
        UnsizedSlicePageGuardBuilder {
            page_table: self,
            vpn_start: VirtualAddress::from_ptr(ptr).to_floor_page_num(),
            ptr: ptr as usize,
            max_len,
            terminator_predicate: Some(|c, idx| c[idx].is_null()),
            exclusive_end: true,
            _marker: PhantomData,
        }
    }
}

pub struct UnsizedSlicePageGuardBuilder<'a, T> {
    page_table: &'a PageTable,
    vpn_start: VirtualPageNum,
    ptr: usize,
    max_len: usize,
    terminator_predicate: Option<fn(&T, usize) -> bool>,
    exclusive_end: bool,
    _marker: PhantomData<T>,
}

impl<'a, T> UnsizedSlicePageGuardBuilder<'a, &'static [T]> {
    pub fn must_have(
        &self,
        flags: PageTableEntryFlags,
    ) -> Option<MustHavePageGuard<'a, &'static [T]>> {
        let mut idx = 0;
        let mut current_ptr = self.ptr;
        let mut current_vpn = self.vpn_start;
        let max_end_va = self.ptr + (self.max_len * core::mem::size_of::<T>());

        let slice = unsafe { core::slice::from_raw_parts(self.ptr as *const T, self.max_len) };

        loop {
            match self.page_table.guard_vpn(current_vpn).must_have(flags) {
                // Still have the permission, check with the predicate
                Some(_) => match &self.terminator_predicate {
                    Some(predicate) => {
                        let page_end_va = current_vpn.end_addr::<VirtualAddress>().as_usize();
                        while current_ptr < Ord::min(max_end_va, page_end_va) {
                            if predicate(&slice, idx) {
                                let len = if self.exclusive_end { idx } else { idx + 1 };
                                return Some(self.build_guard(current_vpn + 1, len));
                            }

                            current_ptr += core::mem::size_of::<T>();
                            idx += 1;
                        }
                    }
                    None => {
                        if max_end_va <= current_vpn.end_addr::<VirtualAddress>().as_usize() {
                            return Some(self.build_guard(current_vpn + 1, self.max_len));
                        }
                    }
                },
                // Reached an end that does not meet the specified permission
                None => {
                    let page_va = current_vpn.start_addr::<VirtualAddress>().as_usize();
                    let mut len = Ord::min(max_end_va, page_va) - self.ptr;

                    len -= len % core::mem::size_of::<T>();

                    return match len {
                        0 => None,
                        _ => Some(self.build_guard(current_vpn, len)),
                    };
                }
            }

            current_vpn += 1;
        }
    }

    fn build_guard(
        &self,
        end_vpn: VirtualPageNum,
        len: usize,
    ) -> MustHavePageGuard<'a, &'static [T]> {
        MustHavePageGuard {
            builder: PageGuardBuilder {
                page_table: self.page_table,
                vpn_range: VirtualPageNumRange::from_start_end(self.vpn_start, end_vpn),
                ptr: self.ptr,
                len,
                _marker: PhantomData,
            },
        }
    }
}

#[allow(unused)]
struct TemporaryModifiedPage {
    page: VirtualPageNum,
    previous: PageTableEntryFlags,
    now: PageTableEntryFlags,
}

pub struct WithPageGuard<'a, T> {
    builder: PageGuardBuilder<'a, T>,
}

pub struct MustHavePageGuard<'a, T> {
    builder: PageGuardBuilder<'a, T>,
}

pub struct PageGuardBuilder<'a, T> {
    page_table: &'a PageTable,
    vpn_range: VirtualPageNumRange,
    ptr: usize,
    len: usize,
    _marker: PhantomData<T>,
}

pub trait IOptionalPageGuardBuilderExtension {
    fn mustbe_user(self) -> Self;

    fn mustbe_readable(self) -> Self;

    fn mustbe_writable(self) -> Self;

    fn mustbe_executable(self) -> Self;
}

impl<T> IOptionalPageGuardBuilderExtension for Option<PageGuardBuilder<'_, T>> {
    fn mustbe_user(self) -> Self {
        self?.must_be_internal(PageTableEntryFlags::User)
    }

    fn mustbe_readable(self) -> Self {
        self?.must_be_internal(PageTableEntryFlags::Readable)
    }

    fn mustbe_writable(self) -> Self {
        self?.must_be_internal(PageTableEntryFlags::Writable)
    }

    fn mustbe_executable(self) -> Self {
        self?.must_be_internal(PageTableEntryFlags::Executable)
    }
}

impl<'a, T> PageGuardBuilder<'a, T> {
    fn must_be_internal(self, mut flags: PageTableEntryFlags) -> Option<Self> {
        // Fast path for rejecting null pointer
        if self.vpn_range.start().as_usize() == 0 {
            return None;
        }

        flags |= PageTableEntryFlags::Valid;
        for page in self.vpn_range.iter() {
            let entry = self.page_table.get_entry_of(page)?;
            if !entry.flags().contains(flags) {
                return None;
            }
        }

        Some(self)
    }

    pub fn must_have(self, flags: PageTableEntryFlags) -> Option<MustHavePageGuard<'a, T>> {
        let this = self.must_be_internal(flags)?;

        Some(MustHavePageGuard { builder: this })
    }

    pub fn mustbe_user(self) -> Option<Self> {
        self.must_be_internal(PageTableEntryFlags::User)
    }

    pub fn mustbe_readable(self) -> Option<Self> {
        self.must_be_internal(PageTableEntryFlags::Readable)
    }

    pub fn mustbe_writable(self) -> Option<Self> {
        self.must_be_internal(PageTableEntryFlags::Writable)
    }

    pub fn mustbe_executable(self) -> Option<Self> {
        self.must_be_internal(PageTableEntryFlags::Executable)
    }

    #[allow(invalid_reference_casting)]
    fn with_internal(self, flags: PageTableEntryFlags) -> Option<WithPageGuard<'a, T>> {
        // Bypass `get_entry_of` as it's unable to handle giant page
        if self.vpn_range.start().as_usize() >= 0xffff_ffc0_0000_0000 / constants::PAGE_SIZE {
            return Some(WithPageGuard { builder: self });
        }

        debug_assert!(
            self.page_table.tracker.is_some(),
            "Page table is not modifiable"
        );
        let tracker = &mut self.page_table.tracker.as_ref().unwrap().lock();

        let mut modified = false;

        for page in self.vpn_range.iter() {
            // TODO: if the page is not mapped, we should do something
            let entry = self.page_table.get_entry_of(page)?;

            let existing_flags = entry.flags();

            if !existing_flags.contains(flags) {
                modified = true;
                *entry |= flags;

                tracker
                    .temporary_modified_pages
                    .entry(page)
                    // merge the flags if the entry already exists
                    .and_modify(|f| {
                        debug_assert!(f.previous == existing_flags);
                        f.now |= flags;
                    })
                    // add entry if not exist
                    .or_insert_with(|| TemporaryModifiedPage {
                        page,
                        previous: existing_flags,
                        now: flags | existing_flags,
                    });
            }
        }

        if modified {
            self.page_table.flush_tlb();
        }

        Some(WithPageGuard { builder: self })
    }
}

pub trait IWithPageGuardBuilder<'a, T> {
    fn with(self, flags: PageTableEntryFlags) -> Option<WithPageGuard<'a, T>>;

    fn with_read(self) -> Option<WithPageGuard<'a, T>>;

    fn with_write(self) -> Option<WithPageGuard<'a, T>>;
}

impl<'a, T> IWithPageGuardBuilder<'a, T> for PageGuardBuilder<'a, T> {
    fn with(self, flags: PageTableEntryFlags) -> Option<WithPageGuard<'a, T>> {
        self.with_internal(flags)
    }

    fn with_read(self) -> Option<WithPageGuard<'a, T>> {
        self.with(PageTableEntryFlags::Readable)
    }

    fn with_write(self) -> Option<WithPageGuard<'a, T>> {
        self.with(PageTableEntryFlags::Readable | PageTableEntryFlags::Writable)
    }
}

impl<'a, T> IWithPageGuardBuilder<'a, T> for Option<PageGuardBuilder<'a, T>> {
    fn with(self, flags: PageTableEntryFlags) -> Option<WithPageGuard<'a, T>> {
        self?.with_internal(flags)
    }

    fn with_read(self) -> Option<WithPageGuard<'a, T>> {
        self.with(PageTableEntryFlags::Readable)
    }

    fn with_write(self) -> Option<WithPageGuard<'a, T>> {
        self.with(PageTableEntryFlags::Readable | PageTableEntryFlags::Writable)
    }
}

impl<'a, T> IWithPageGuardBuilder<'a, T> for Option<MustHavePageGuard<'a, T>> {
    fn with(self, flags: PageTableEntryFlags) -> Option<WithPageGuard<'a, T>> {
        self?.builder.with_internal(flags)
    }

    fn with_read(self) -> Option<WithPageGuard<'a, T>> {
        self.with(PageTableEntryFlags::Readable)
    }

    fn with_write(self) -> Option<WithPageGuard<'a, T>> {
        self.with(PageTableEntryFlags::Readable | PageTableEntryFlags::Writable)
    }
}

trait IHasPageGuardBuilder<'a, TValue> {
    fn ptr(&self) -> usize;
    fn len(&self) -> usize;
}

impl<'a, TValue> IHasPageGuardBuilder<'a, TValue> for WithPageGuard<'a, TValue> {
    fn ptr(&self) -> usize {
        self.builder.ptr
    }

    fn len(&self) -> usize {
        self.builder.len
    }
}

impl<'a, TValue> IHasPageGuardBuilder<'a, TValue> for MustHavePageGuard<'a, TValue> {
    fn ptr(&self) -> usize {
        self.builder.ptr
    }

    fn len(&self) -> usize {
        self.builder.len
    }
}

// implementation on `dyn IHasPermissionGuardBuilder` only works for interface instances
// So we need to implement for each concrete type
impl<T> AsMut<T> for MustHavePageGuard<'_, &'static T> {
    fn as_mut(&mut self) -> &'static mut T {
        unsafe { &mut *(self.ptr() as *mut T) }
    }
}

impl<T> Deref for MustHavePageGuard<'_, &'static T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.ptr() as *const T) }
    }
}

impl<T> AsMut<[T]> for MustHavePageGuard<'_, &'static [T]> {
    fn as_mut(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.ptr() as *mut T, self.len()) }
    }
}

impl<T> Deref for MustHavePageGuard<'_, &'static [T]> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { slice::from_raw_parts(self.ptr() as *const T, self.len()) }
    }
}

impl<T> AsMut<T> for WithPageGuard<'_, &'static T> {
    fn as_mut(&mut self) -> &'static mut T {
        unsafe { &mut *(self.ptr() as *mut T) }
    }
}

impl<T> Deref for WithPageGuard<'_, &'static T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.ptr() as *const T) }
    }
}

impl<T> DerefMut for WithPageGuard<'_, &'static T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.ptr() as *mut T) }
    }
}

impl<T> Deref for WithPageGuard<'_, &'static [T]> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { slice::from_raw_parts(self.ptr() as *const T, self.len()) }
    }
}

impl<T> DerefMut for WithPageGuard<'_, &'static [T]> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { slice::from_raw_parts_mut(self.ptr() as *mut T, self.len()) }
    }
}
