use abstractions::IUsizeAlias;
use address::{IPageNum, PhysicalAddress, PhysicalPageNum};
use core::{mem::forget, ptr::NonNull};

pub struct VirtHal;

unsafe impl virtio_drivers::Hal for VirtHal {
    fn dma_alloc(
        pages: usize,
        _direction: virtio_drivers::BufferDirection,
    ) -> (virtio_drivers::PhysAddr, core::ptr::NonNull<u8>) {
        let frames = allocation::alloc_contiguous(pages)
            .expect("Failed to allocate contiguous frames for Virt DMA");
        let frame_range = frames.to_range();
        forget(frames); // Prevent deallocation

        let paddr = frame_range.start().start_addr().as_usize();
        let vaddr =
            unsafe { NonNull::new_unchecked((paddr | constants::VIRT_ADDR_OFFSET) as *mut u8) };

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
        debug_assert!(paddr | constants::VIRT_ADDR_OFFSET == vaddr.as_ptr() as usize);

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
        // debug_assert_eq!(address & constants::VIRT_ADDR_OFFSET, constants::VIRT_ADDR_OFFSET, "{:#018x}", address);

        // We can even return the virtual as the whole kernel space is mapped to the higher half
        // And we don't have to worry about the physical
        (address & constants::PHYS_ADDR_MASK) as virtio_drivers::PhysAddr
    }

    unsafe fn unshare(
        _paddr: virtio_drivers::PhysAddr,
        _buffer: core::ptr::NonNull<[u8]>,
        _direction: virtio_drivers::BufferDirection,
    ) {
        // _paddr may not be the start of a frame, so the assertion is not correct
        // #[cfg(debug_assertions)]
        // {
        //     // Ensure that the address is a virtual address
        //     debug_assert!(_paddr & constants::VIRT_ADDR_OFFSET == constants::VIRT_ADDR_OFFSET);
        // }

        // Do nothing
    }
}
