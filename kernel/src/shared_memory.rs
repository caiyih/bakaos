use address::{IPageNum, VirtualPageNum};
use alloc::{
    collections::btree_map::{self, BTreeMap},
    sync::Arc,
    vec::Vec,
};
use allocation::TrackedFrame;
use hermit_sync::SpinMutex;
use page_table::{GenericMappingFlags, PageSize};
use tasks::TaskControlBlock;

static SHARED_MEMORY: SpinMutex<BTreeMap<usize, Vec<TrackedFrame>>> =
    SpinMutex::new(BTreeMap::new());

pub fn get_last_created() -> Result<usize, usize> {
    let inner = SHARED_MEMORY.lock();

    match inner.keys().max() {
        Some(key) => Ok(*key),
        None => Err(1),
    }
}

pub fn is_shm_existing(key: usize) -> bool {
    SHARED_MEMORY.lock().contains_key(&key)
}

pub fn allocate_at(key: usize, size: usize) -> bool {
    match SHARED_MEMORY.lock().entry(key) {
        btree_map::Entry::Occupied(_) => false,
        btree_map::Entry::Vacant(vacant_entry) => {
            let count = size.div_ceil(constants::PAGE_SIZE);
            let frames = allocation::alloc_frames(count).unwrap();

            vacant_entry.insert(frames);

            true
        }
    }
}

// pub fn deallocate_at(key: usize) -> bool {
//     // FIXME: do we have to unmap pages for processes?
//     SHARED_MEMORY.lock().remove(&key).is_some()
// }

pub fn apply_mapping_for(tcb: &Arc<TaskControlBlock>, key: usize) -> Option<VirtualPageNum> {
    match SHARED_MEMORY.lock().get(&key) {
        None => None,
        Some(frames) => {
            // VRWEUAD
            const PAGE_FLAG: GenericMappingFlags = GenericMappingFlags::all();

            let mut pcb = tcb.pcb.lock();

            // prevent overlapping
            let vpn_range = pcb
                .mmaps
                .allocate_records(frames.len() * constants::PAGE_SIZE, None)
                .unwrap();

            let pt = pcb.memory_space.page_table_mut();

            debug_assert!(vpn_range.page_count() >= frames.len());

            let start_page = vpn_range.start();
            for (i, frame) in frames.iter().enumerate() {
                pt.map_single(
                    (start_page + i).start_addr(),
                    frame.ppn().start_addr(),
                    PageSize::_4K,
                    PAGE_FLAG,
                )
                .unwrap();
            }

            Some(start_page)
        }
    }
}
