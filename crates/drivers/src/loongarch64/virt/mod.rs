use alloc::sync::Arc;
use timing::TimeSpec;

use crate::{BlockDeviceInode, IMachine};

#[derive(Clone, Copy)]
pub struct VirtMachine;

impl IMachine for VirtMachine {
    fn name(&self) -> &'static str {
        "QEMU Virt Machine(LoongArch64)"
    }

    fn query_performance_frequency(&self) -> u64 {
        100_000_000
    }

    fn mmio(&self) -> &[(usize, usize)] {
        &[
            (0x100E_0000, 0x0000_1000), // GED
            (0x1FE0_0000, 0x0000_1000), // UART
            (0x2000_0000, 0x1000_0000), // PCI
            (0x4000_0000, 0x0002_0000), // PCI RANGES
        ]
    }

    fn memory_end(&self) -> usize {
        // 128M
        0x8800_0000
    }

    fn query_performance_counter(&self) -> usize {
        // TODO: Implement this
        0
    }

    fn get_rtc_offset(&self) -> timing::TimeSpec {
        TimeSpec::zero()
    }

    fn create_block_device_at(&self, _device_id: usize) -> Arc<BlockDeviceInode> {
        todo!()
    }
}
