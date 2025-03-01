use alloc::sync::Arc;
use timing::TimeSpec;

use crate::block::BlockDeviceInode;

pub trait IMachine {
    // Board metadata
    fn name(&self) -> &'static str;
    fn clock_freq(&self) -> u64;
    fn mmio(&self) -> &[(usize, usize)];
    fn memory_end(&self) -> usize;

    fn get_board_tick(&self) -> usize;

    #[allow(unused)]
    fn block_sleep(&self, ms: usize) {
        let start = self.get_board_tick();
        let end = start + ms * self.clock_freq() as usize / 1000;

        while self.get_board_tick() < end {
            core::hint::spin_loop();
        }
    }

    fn bus0(&self) -> usize;
    fn bus_width(&self) -> usize;

    fn mmc_driver(&self, device_id: usize) -> usize {
        self.bus0() + device_id * self.bus_width()
    }

    #[inline(always)]
    fn machine_uptime(&self) -> u64 {
        self.get_board_tick() as u64 / self.clock_freq()
    }

    fn get_rtc_offset(&self) -> TimeSpec;

    fn create_block_device_at(&self, device_id: usize) -> Arc<BlockDeviceInode>;
}
