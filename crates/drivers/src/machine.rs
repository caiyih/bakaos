use alloc::sync::Arc;
use timing::TimeSpec;

use crate::block::BlockDeviceInode;

pub trait IMachine {
    // Board metadata
    fn name(&self) -> &'static str;
    fn query_performance_frequency(&self) -> u64;
    fn mmio(&self) -> &[(usize, usize)];
    fn memory_end(&self) -> usize;

    fn query_performance_counter(&self) -> usize;

    #[allow(unused)]
    fn block_sleep(&self, ms: usize) {
        let start = self.query_performance_counter();
        let end = start + ms * self.query_performance_frequency() as usize / 1000;

        while self.query_performance_counter() < end {
            core::hint::spin_loop();
        }
    }

    #[inline(always)]
    fn machine_uptime(&self) -> u64 {
        self.query_performance_counter() as u64 / self.query_performance_frequency()
    }

    fn get_rtc_offset(&self) -> TimeSpec;

    fn create_block_device_at(&self, device_id: usize) -> Arc<BlockDeviceInode>;
}
