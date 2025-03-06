mod block;

pub use block::*;

use abstractions::IUsizeAlias;
use address::IConvertablePhysicalAddress;
use address::PhysicalAddress;
use alloc::{boxed::Box, sync::Arc};
use timing::{TimeSpec, NSEC_PER_SEC};

use crate::{BlockDeviceInode, IMachine};

#[derive(Clone, Copy)]
pub struct VF2Machine;

impl VF2Machine {
    const fn bus0(&self) -> usize {
        0x16020000
    }

    const fn bus_width(&self) -> usize {
        0x1_0000
    }
}

impl IMachine for VF2Machine {
    fn name(&self) -> &'static str {
        "StarFive VisionFive 2"
    }

    fn query_performance_frequency(&self) -> u64 {
        4_000_000
    }

    fn mmio(&self) -> &[(usize, usize)] {
        &[
            (0x10000000, 0x10000),
            (0x16020000, 0x10000),
            (0x17040000, 0x10000),
        ]
    }

    fn memory_end(&self) -> usize {
        // 4 GB
        0x1_80000000
    }

    fn create_block_device_at(&self, device_id: usize) -> Arc<BlockDeviceInode> {
        let mmio_pa = PhysicalAddress::from_usize(self.bus0() + device_id * self.bus_width());
        let mmio = VisionFive2SdMMIO::new(mmio_pa.to_high_virtual());

        BlockDeviceInode::new(Box::new(VisionFive2Disk::new(mmio)))
    }

    fn query_performance_counter(&self) -> usize {
        platform_specific::time()
    }

    fn get_rtc_offset(&self) -> TimeSpec {
        // TODO: this is a temporary implementation, only mmio is comfirmerd
        // This implementation is based on QEMU virt's implementation, goldfish_rtc
        // Need to figure out the layout of the RTC registers

        // mmio, width
        // 0x17040000, 0x10000
        let mmio = PhysicalAddress::from_usize(0x17040000);
        let mmio = mmio.to_high_virtual();

        let low = unsafe { mmio.as_ptr::<u32>().read_volatile() };
        let tick = self.query_performance_counter();

        let high = unsafe { mmio.as_ptr::<u32>().add(1).read_volatile() };

        let rtc_ns = ((high as u64) << 32) | low as u64;

        let reg_time = TimeSpec::from_ticks(tick as i64, self.query_performance_frequency());
        let rtc_time = TimeSpec::from_ticks(rtc_ns as i64, NSEC_PER_SEC as u64);

        rtc_time - reg_time
    }
}
