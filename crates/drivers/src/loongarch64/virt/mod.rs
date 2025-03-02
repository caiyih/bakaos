use timing::TimeSpec;

use crate::IMachine;

#[derive(Clone, Copy)]
pub struct VirtMachine;

impl IMachine for VirtMachine {
    fn name(&self) -> &'static str {
        "QEMU Virt Machine(LoongArch64)"
    }

    fn clock_freq(&self) -> u64 {
        // TODO: figure out the correct value
        12_500_000
    }

    fn mmio(&self) -> &[(usize, usize)] {
        &[]
    }

    fn memory_end(&self) -> usize {
        // 128M
        0x8800_0000
    }

    fn get_board_tick(&self) -> usize {
        // TODO: figure out the correct value
        12_500_000
    }

    fn bus0(&self) -> usize {
        0
    }

    fn bus_width(&self) -> usize {
        0
    }

    fn get_rtc_offset(&self) -> timing::TimeSpec {
        TimeSpec::zero()
    }

    fn create_block_device_at(
        &self,
        _device_id: usize,
    ) -> alloc::sync::Arc<crate::BlockDeviceInode> {
        todo!()
    }
}
