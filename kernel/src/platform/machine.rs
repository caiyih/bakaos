use riscv::register::time;

pub trait IMachine {
    // Board metadata
    fn name(&self) -> &'static str;
    fn clock_freq(&self) -> u64;
    fn mmio(&self) -> &[(usize, usize)];
    fn memory_end(&self) -> usize;

    #[inline(always)]
    fn get_board_tick(&self) -> usize {
        time::read()
    }

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

    fn tick_to_ms(&self, tick: usize) -> u64 {
        (tick as u64) * 1000 / self.clock_freq()
    }

    fn tick_to_timestamp(&self, tick: u64) -> u64 {
        tick / self.clock_freq()
    }

    #[inline(always)]
    fn current_timestamp(&self) -> u64 {
        self.tick_to_timestamp(self.get_board_tick() as u64)
    }

    #[inline(always)]
    fn currrent_time_ms(&self) -> u64 {
        self.tick_to_ms(self.get_board_tick())
    }
}
