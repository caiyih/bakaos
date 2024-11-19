use address::{IAddressBase, IPageNum, PhysicalAddress, PhysicalPageNum};
use core::{mem::forget, ptr::NonNull};
use virtio_drivers::{device::blk::VirtIOBlk, transport::mmio::MmioTransport};

use crate::IDiskDevice;

pub const SECTOR_SIZE: usize = 512;

pub type VirtioDiskDriver = VirtioDisk<VirtHal>;

pub struct VirtioDisk<THal>
where
    THal: virtio_drivers::Hal,
{
    sector: usize,
    offset: usize,
    virtio_blk: VirtIOBlk<THal, MmioTransport>,
}

impl<T> VirtioDisk<T>
where
    T: virtio_drivers::Hal,
{
    pub fn new(virtio_blk: VirtIOBlk<T, MmioTransport>) -> Self {
        VirtioDisk {
            sector: 0,
            offset: 0,
            virtio_blk,
        }
    }
}

impl<T> IDiskDevice for VirtioDisk<T>
where
    T: virtio_drivers::Hal,
{
    fn read_blocks(&mut self, buf: &mut [u8]) {
        self.virtio_blk
            .read_blocks(self.sector, buf)
            .expect("Error occurred when reading VirtIOBlk");
    }

    fn write_blocks(&mut self, buf: &[u8]) {
        self.virtio_blk
            .write_blocks(self.sector, buf)
            .expect("Error occurred when writing VirtIOBlk");
    }

    fn get_position(&self) -> usize {
        self.sector * SECTOR_SIZE + self.offset
    }

    fn set_position(&mut self, position: usize) {
        self.sector = position / SECTOR_SIZE;
        self.offset = position % SECTOR_SIZE;
    }

    fn move_cursor(&mut self, amount: usize) {
        self.set_position(self.get_position() + amount)
    }
}

pub struct VirtHal;

unsafe impl virtio_drivers::Hal for VirtHal {
    fn dma_alloc(
        pages: usize,
        _direction: virtio_drivers::BufferDirection,
    ) -> (virtio_drivers::PhysAddr, core::ptr::NonNull<u8>) {
        let frames = allocation::alloc_contiguous(pages)
            .expect("Failed to allocate contiguous frames for Virt DMA")
            .to_range();

        let paddr = frames.start().start_addr::<PhysicalAddress>().as_usize();
        let vaddr =
            unsafe { NonNull::new_unchecked((paddr | constants::VIRT_ADDR_OFFSET) as *mut u8) };

        forget(frames); // Prevent deallocation

        (paddr, vaddr)
    }

    unsafe fn dma_dealloc(
        paddr: virtio_drivers::PhysAddr,
        vaddr: core::ptr::NonNull<u8>,
        pages: usize,
    ) -> i32 {
        // ensure paddr is a physical address
        debug_assert!(paddr & constants::VIRT_ADDR_OFFSET == 0);
        // ensure paddr is properly mapped to vaddr
        debug_assert!(paddr & constants::VIRT_ADDR_OFFSET == vaddr.as_ptr() as usize);

        let ppn = PhysicalPageNum::from_addr_floor(PhysicalAddress::from_usize(paddr));
        for i in 0..pages {
            allocation::dealloc_frame_unchecked(ppn + i);
        }
        0
    }

    unsafe fn mmio_phys_to_virt(
        paddr: virtio_drivers::PhysAddr,
        _size: usize,
    ) -> core::ptr::NonNull<u8> {
        // Refer to kernel virtual memory layout for more details
        NonNull::new_unchecked((paddr | constants::VIRT_ADDR_OFFSET) as *mut u8)
    }

    unsafe fn share(
        buffer: core::ptr::NonNull<[u8]>,
        _direction: virtio_drivers::BufferDirection,
    ) -> virtio_drivers::PhysAddr {
        let address = buffer.as_ptr() as *mut u8 as usize;

        // Ensure that the address is a virtual address
        debug_assert!(address & constants::VIRT_ADDR_OFFSET == constants::VIRT_ADDR_OFFSET);

        // We can even return the virtual as the whole kernel space is mapped to the higher half
        // And we don't have to worry about the physical
        (address & constants::PHYS_ADDR_MASK) as virtio_drivers::PhysAddr
    }

    unsafe fn unshare(
        _paddr: virtio_drivers::PhysAddr,
        _buffer: core::ptr::NonNull<[u8]>,
        _direction: virtio_drivers::BufferDirection,
    ) {
        #[cfg(debug_assertions)]
        {
            // Ensure that the address is a virtual address
            debug_assert!(_paddr & constants::VIRT_ADDR_OFFSET == constants::VIRT_ADDR_OFFSET);
        }

        // Do nothing
    }
}
