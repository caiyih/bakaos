use address::{VirtualPageNum, VirtualPageNumRange};
use alloc::{collections::btree_map::BTreeMap, sync::Arc};
use allocation_abstractions::{FrameDesc, IFrameAllocator};
use hermit_sync::SpinMutex;
use mmu_abstractions::GenericMappingFlags;

use crate::{AreaType, MapType};

pub struct MappingArea {
    pub range: VirtualPageNumRange,
    pub area_type: AreaType,
    pub map_type: MapType,
    pub permissions: GenericMappingFlags,
    pub allocation: Option<MappingAreaAllocation>,
}

impl MappingArea {
    pub fn range(&self) -> VirtualPageNumRange {
        self.range
    }

    pub fn permissions(&self) -> GenericMappingFlags {
        self.permissions
    }

    pub fn map_type(&self) -> AreaType {
        self.area_type
    }

    pub fn new(
        range: VirtualPageNumRange,
        area_type: AreaType,
        map_type: MapType,
        permissions: GenericMappingFlags,
        allocation: Option<MappingAreaAllocation>,
    ) -> Self {
        Self {
            range,
            area_type,
            map_type,
            permissions,
            allocation,
        }
    }

    pub fn clone_from(area: &MappingArea) -> Self {
        Self {
            range: area.range,
            area_type: area.area_type,
            map_type: area.map_type,
            permissions: area.permissions,
            allocation: None,
        }
    }

    pub fn contains(&self, vpn: VirtualPageNum) -> bool {
        self.range.contains(vpn)
    }
}

impl alloc::fmt::Debug for MappingArea {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MappingArea")
            .field("range", &self.range)
            .field("area_type", &self.area_type)
            .field("map_type", &self.map_type)
            .field("permissions", &self.permissions)
            .field("allocation", &self.allocation.is_some())
            .finish()
    }
}

pub struct MappingAreaAllocation {
    pub allocator: Arc<SpinMutex<dyn IFrameAllocator>>,
    pub frames: BTreeMap<VirtualPageNum, FrameDesc>,
}

impl MappingAreaAllocation {
    pub fn empty(allocator: Arc<SpinMutex<dyn IFrameAllocator>>) -> Self {
        Self {
            allocator,
            frames: BTreeMap::new(),
        }
    }
}

impl Drop for MappingAreaAllocation {
    fn drop(&mut self) {
        while let Some((_, frame)) = self.frames.pop_first() {
            self.allocator.lock().dealloc(frame);
        }
    }
}
