use abstractions::IUsizeAlias;
use address::PhysicalAddress;
use alloc::boxed::Box;
use drivers::{DiskDriver, VisionFive2Disk};
use filesystem::{Ext4FileSystem, Fat32FileSystem};

use super::machine::IMachine;

#[derive(Clone, Copy)]
pub struct VF2Machine;

impl IMachine for VF2Machine {
    fn name(&self) -> &'static str {
        "StarFive VisionFive 2"
    }

    fn clock_freq(&self) -> u64 {
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

    fn bus0(&self) -> usize {
        0x16020000
    }

    fn bus_width(&self) -> usize {
        0x1_0000
    }

    fn create_fat32_filesystem_at_bus(&self, device_id: usize) -> filesystem::Fat32FileSystem {
        Fat32FileSystem::new(self.create_block_driver_at_bus(device_id))
            .expect("Failed to initialize FAT32 filesystem on VisionFive2Disk")
    }

    fn create_ext4_filesystem_at_bus(&self, device_id: usize) -> filesystem::Ext4FileSystem {
        Ext4FileSystem::new(self.create_block_driver_at_bus(device_id))
            .expect("Failed to initialize EXT4 filesystem on VisionFive2Disk")
    }
}

impl VF2Machine {
    fn create_block_driver_at_bus(&self, device_id: usize) -> DiskDriver {
        let mmio_pa = PhysicalAddress::from_usize(self.bus0() + device_id * self.bus_width());
        let mmio = drivers::VisionFive2SdMMIO::new(mmio_pa.to_high_virtual());

        DiskDriver::new(Box::new(VisionFive2Disk::new(mmio)))
    }
}
