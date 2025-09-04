use core::cell::OnceCell;

use abstractions::IUsizeAlias;
use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};

use crate::{AreaType, MapType, MappingArea, MappingAreaAllocation};
use address::{
    IAddressBase, IPageNum, IToPageNum, PhysicalAddress, VirtualAddress, VirtualAddressRange,
    VirtualPageNum, VirtualPageNumRange,
};
use allocation_abstractions::IFrameAllocator;
use hermit_sync::SpinMutex;
use mmu_abstractions::{GenericMappingFlags, PageSize, IMMU};

pub struct MemorySpace {
    mmu: Arc<SpinMutex<dyn IMMU>>,
    mapping_areas: Vec<MappingArea>,
    attr: OnceCell<MemorySpaceAttribute>,
    allocator: Arc<SpinMutex<dyn IFrameAllocator>>,
}

#[derive(Debug, Clone, Copy)]
pub struct MemorySpaceAttribute {
    pub brk_area_idx: usize,
    pub brk_start: VirtualAddress,
    pub stack_guard_base: VirtualAddressRange,
    pub stack_range: VirtualAddressRange,
    pub stack_guard_top: VirtualAddressRange,
    pub elf_area: VirtualAddressRange,
    pub signal_trampoline: VirtualPageNum,
}

impl Default for MemorySpaceAttribute {
    /// Returns a MemorySpaceAttribute populated with sentinel (empty/invalid) defaults.
    ///
    /// The default instance uses sentinel values to represent an uninitialized memory-space:
    /// - `brk_area_idx` is `usize::MAX` (invalid index),
    /// - all virtual address/range fields are set to `VirtualAddress::null()` or a
    ///   `VirtualAddressRange` built from null addresses,
    /// - `signal_trampoline` is page 0.
    ///
    /// # Examples
    ///
    /// ```
    /// let attr = MemorySpaceAttribute::default();
    /// assert_eq!(attr.brk_area_idx, usize::MAX);
    /// assert!(attr.brk_start.is_null());
    /// assert_eq!(attr.signal_trampoline.as_usize(), 0);
    /// ```
    fn default() -> Self {
        Self {
            brk_area_idx: usize::MAX,
            brk_start: VirtualAddress::null(),
            stack_guard_base: VirtualAddressRange::from_start_end(
                VirtualAddress::null(),
                VirtualAddress::null(),
            ),
            stack_range: VirtualAddressRange::from_start_end(
                VirtualAddress::null(),
                VirtualAddress::null(),
            ),
            stack_guard_top: VirtualAddressRange::from_start_end(
                VirtualAddress::null(),
                VirtualAddress::null(),
            ),
            elf_area: VirtualAddressRange::from_start_end(
                VirtualAddress::null(),
                VirtualAddress::null(),
            ),
            signal_trampoline: VirtualPageNum::from_usize(0),
        }
    }
}

impl MemorySpace {
    pub fn mappings(&self) -> &[MappingArea] {
        &self.mapping_areas
    }

    pub fn alloc_and_map_area(&mut self, mut area: MappingArea) {
        debug_assert!(area.allocation.is_none());

        let mut alloc = self.create_empty_area_allocation();

        {
            for vpn in area.range().iter() {
                let frame = alloc.allocator.lock().alloc_frame().unwrap();
                let paddr = frame.0;

                alloc.frames.insert(vpn, frame);

                self.mmu
                    .lock()
                    .map_single(vpn.start_addr(), paddr, PageSize::_4K, area.permissions())
                    .unwrap();
            }
        }

        area.allocation = Some(alloc);
        self.mapping_areas.push(area);
    }

    pub fn map_area(&mut self, area: MappingArea) {
        debug_assert!(area.allocation.is_some());
        debug_assert!(Arc::ptr_eq(
            &area.allocation.as_ref().unwrap().allocator,
            &self.allocator
        ));

        self.mapping_areas.push(area);
    }

    pub fn unmap_first_area_that(&mut self, predicate: &impl Fn(&MappingArea) -> bool) -> bool {
        match self.mapping_areas.iter().position(predicate) {
            Some(index) => {
                let area = self.mapping_areas.remove(index);
                for vpn in area.range.iter() {
                    self.mmu.lock().unmap_single(vpn.start_addr()).unwrap();
                }
                // Drop area to release allocated frames
                true
            }
            None => false,
        }
    }

    pub fn unmap_all_areas_that(&mut self, predicate: impl Fn(&MappingArea) -> bool) {
        while self.unmap_first_area_that(&predicate) {
            // do nothing
        }
    }

    pub fn unmap_area_starts_with(&mut self, vpn: VirtualPageNum) -> bool {
        self.unmap_first_area_that(&|area| area.range.start() == vpn)
    }
}

impl MemorySpace {
    pub fn attr(&self) -> &MemorySpaceAttribute {
        self.attr.get().unwrap()
    }

    pub fn brk_start(&self) -> VirtualAddress {
        self.attr().brk_start
    }

    pub fn brk_page_range(&self) -> VirtualPageNumRange {
        self.mapping_areas[self.brk_area_idx()].range()
    }

    pub fn brk_area_idx(&self) -> usize {
        self.attr().brk_area_idx
    }

    pub fn increase_brk(&mut self, new_end_vpn: VirtualPageNum) -> Result<(), &str> {
        let brk_idx = self.brk_area_idx();

        let old_end_vpn;

        {
            let brk_area = &mut self.mapping_areas[brk_idx];

            if new_end_vpn < brk_area.range.start() {
                return Err("New end is less than the current start");
            }

            old_end_vpn = brk_area.range.end();
        }

        let page_count = new_end_vpn.diff_page_count(old_end_vpn);

        if page_count == 0 {
            return Ok(());
        }

        let increased_range =
            VirtualPageNumRange::from_start_count(old_end_vpn, page_count as usize);

        for vpn in increased_range.iter() {
            let frame = self.allocator.lock().alloc_frame().unwrap();
            let paddr = frame.0;

            let area = &mut self.mapping_areas[brk_idx];

            area.allocation.as_mut().unwrap().frames.insert(vpn, frame);

            self.mmu
                .lock()
                .map_single(vpn.start_addr(), paddr, PageSize::_4K, area.permissions())
                .unwrap();
        }

        let brk_area = &mut self.mapping_areas[brk_idx];

        brk_area.range = VirtualPageNumRange::from_start_end(brk_area.range.start(), new_end_vpn);

        Ok(())
    }
}

impl MemorySpace {
    pub fn new(
        mmu: Arc<SpinMutex<dyn IMMU>>,
        allocator: Arc<SpinMutex<dyn IFrameAllocator>>,
    ) -> Self {
        Self {
            mmu,
            mapping_areas: Vec::new(),
            attr: OnceCell::new(),
            allocator,
        }
    }

    pub fn mmu(&self) -> &Arc<SpinMutex<dyn IMMU>> {
        &self.mmu
    }

    pub fn allocator(&self) -> &Arc<SpinMutex<dyn IFrameAllocator>> {
        &self.allocator
    }

    pub(crate) fn create_empty_area_allocation(&self) -> MappingAreaAllocation {
        MappingAreaAllocation {
            allocator: self.allocator.clone(),
            frames: BTreeMap::new(),
        }
    }

    /// Initialize the memory space's attribute value
    ///
    /// # Safety
    ///
    /// The function is NOT thread safe.
    pub unsafe fn init(&mut self, attr: MemorySpaceAttribute) {
        self.attr.set(attr).unwrap();
    }
}

impl MemorySpace {
    // Clone the existing memory space
    pub fn clone_existing(
        them: &MemorySpace,
        mmu: Arc<SpinMutex<dyn IMMU>>,
        allocator: Option<Arc<SpinMutex<dyn IFrameAllocator>>>,
    ) -> Self {
        let mut this = Self::new(mmu, allocator.unwrap_or(them.allocator().clone()));

        let mut buffer: [u8; constants::PAGE_SIZE] = [0; constants::PAGE_SIZE];

        for area in them.mapping_areas.iter() {
            let my_area = MappingArea::clone_from(area);
            this.alloc_and_map_area(my_area);

            // Copy datas through high half address
            for src_page in area.range.iter() {
                let their_pt = them.mmu().lock();

                their_pt
                    .read_bytes(src_page.start_addr(), &mut buffer)
                    .unwrap();

                this.mmu()
                    .lock()
                    .write_bytes(src_page.start_addr(), &buffer)
                    .unwrap();
            }
        }

        *this.attr.get_mut().unwrap() = *them.attr();

        this
    }

    pub fn signal_trampoline(&self) -> VirtualAddress {
        self.attr().signal_trampoline.start_addr()
    }

    pub fn register_signal_trampoline(&mut self, sigreturn: PhysicalAddress) {
        const PERMISSIONS: GenericMappingFlags = GenericMappingFlags::Kernel
            .union(GenericMappingFlags::User)
            .union(GenericMappingFlags::Readable)
            .union(GenericMappingFlags::Executable);

        log::info!("Registering signal trampoline at {:?}", sigreturn);

        assert!(self.signal_trampoline() != VirtualAddress::null());

        let trampoline_page = self.signal_trampoline();

        self.mmu
            .lock()
            .map_single(trampoline_page, sigreturn, PageSize::_4K, PERMISSIONS)
            .unwrap();

        self.mapping_areas.push(MappingArea::new(
            VirtualPageNumRange::from_start_count(trampoline_page.to_floor_page_num(), 1),
            AreaType::SignalTrampoline,
            MapType::Framed,
            PERMISSIONS,
            None,
        ));
    }
}
