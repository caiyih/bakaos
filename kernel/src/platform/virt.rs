use alloc::boxed::Box;
use core::ptr::NonNull;
use drivers::virt::{VirtHal, VirtioDisk};
use filesystem::Fat32FileSystem;
use virtio_drivers::{
    device::blk::VirtIOBlk,
    transport::mmio::{MmioTransport, VirtIOHeader},
};

use super::machine::IMachine;

#[derive(Clone, Copy)]
pub struct VirtBoard;

impl IMachine for VirtBoard {
    fn name(&self) -> &'static str {
        "QEMU Virt Machine"
    }

    fn clock_freq(&self) -> u64 {
        12_500_000
    }

    fn mmio(&self) -> &[(usize, usize)] {
        &[
            (0x0010_0000, 0x00_2000), // VIRT_TEST/RTC  in virt machine
            (0x2000000, 0x10000),     // core local interrupter (CLINT)
            (0xc000000, 0x210000),    // VIRT_PLIC in virt machine
            (0x10000000, 0x9000),     // VIRT_UART0 with GPU  in virt machine
        ]
    }

    fn memory_end(&self) -> usize {
        0x8800_0000
    }

    fn bus0(&self) -> usize {
        0x1000_1000
    }

    fn bus_width(&self) -> usize {
        0x1000
    }

    fn create_fat32_filesystem_at_bus(&self, device_id: usize) -> filesystem::Fat32FileSystem {
        let mmio_pa = self.mmc_driver(device_id);
        let mmio_va = mmio_pa | constants::VIRT_ADDR_OFFSET;

        let ptr = unsafe { NonNull::new_unchecked(mmio_va as *mut VirtIOHeader) };
        let mmio_transport =
            unsafe { MmioTransport::new(ptr).expect("Failed to initialize virtio mmio transport") };
        let virt_blk = VirtIOBlk::<VirtHal, _>::new(mmio_transport)
            .expect("Failed to initialize virtio block device");
        let virt_disk = VirtioDisk::new(virt_blk);

        Fat32FileSystem::new(Box::new(virt_disk))
            .expect("Failed to initialize FAT32 filesystem on VirtIOBlk")
    }
}
