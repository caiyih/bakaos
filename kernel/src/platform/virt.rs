use abstractions::IUsizeAlias;
use address::PhysicalAddress;
use alloc::boxed::Box;
use core::ptr::NonNull;
use drivers::{
    virt::{VirtHal, VirtioDisk},
    BlockDeviceInode,
};
use riscv::register::time;
use timing::{TimeSpec, NSEC_PER_SEC};
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

    fn create_block_device_at(
        &self,
        device_id: usize,
    ) -> alloc::sync::Arc<dyn filesystem_abstractions::IInode> {
        let mmio_pa = self.mmc_driver(device_id);
        let mmio_va = mmio_pa | constants::VIRT_ADDR_OFFSET;

        let ptr = unsafe { NonNull::new_unchecked(mmio_va as *mut VirtIOHeader) };
        let mmio_transport = unsafe {
            MmioTransport::new(ptr, self.bus_width())
                .expect("Failed to initialize virtio mmio transport")
        };
        let virt_blk = VirtIOBlk::<VirtHal, _>::new(mmio_transport)
            .expect("Failed to initialize virtio block device");
        let virt_disk = VirtioDisk::new(virt_blk);

        BlockDeviceInode::new(Box::new(virt_disk))
    }

    fn get_rtc_offset(&self) -> TimeSpec {
        let mmio = PhysicalAddress::from_usize(0x101000);
        let mmio = mmio.to_high_virtual();

        let low = unsafe { mmio.as_ptr::<u32>().read_volatile() };
        let tick = time::read();

        let high = unsafe { mmio.as_ptr::<u32>().add(1).read_volatile() };

        let rtc_ns = ((high as u64) << 32) | low as u64;

        let reg_time = TimeSpec::from_ticks(tick as i64, self.clock_freq());
        let rtc_time = TimeSpec::from_ticks(rtc_ns as i64, NSEC_PER_SEC as u64);

        rtc_time - reg_time
    }
}
