use address::VirtualAddress;
use visionfive2_sd::Vf2SdDriver;

use crate::IRawDiskDevice;

pub struct VisionFive2SdMMIO {
    mmio: VirtualAddress,
}

impl VisionFive2SdMMIO {
    pub fn new(mmio: VirtualAddress) -> VisionFive2SdMMIO {
        VisionFive2SdMMIO { mmio }
    }
}

impl visionfive2_sd::SDIo for VisionFive2SdMMIO {
    fn read_data_at(&self, offset: usize) -> u64 {
        unsafe {
            let addr = (self.mmio + offset).as_ptr::<u64>();
            addr.read_volatile()
        }
    }

    fn read_reg_at(&self, offset: usize) -> u32 {
        unsafe {
            let addr = (self.mmio + offset).as_ptr::<u32>();
            addr.read_volatile()
        }
    }

    fn write_data_at(&mut self, offset: usize, val: u64) {
        unsafe {
            let addr = (self.mmio + offset).as_mut_ptr::<u64>();
            addr.write_volatile(val);
        }
    }

    fn write_reg_at(&mut self, offset: usize, val: u32) {
        unsafe {
            let addr = (self.mmio + offset).as_mut_ptr::<u32>();
            addr.write_volatile(val);
        }
    }
}

struct SleepHelper;

impl SleepHelper {
    unsafe fn read_tick() -> usize {
        #[cfg(not(target_arch = "riscv64"))]
        panic!("read_tick not implemented for this target");

        #[cfg(target_arch = "riscv64")]
        {
            let tick;
            core::arch::asm!("csrr {}, time", out(reg) tick);
            tick
        }
    }

    const VF2_FREQ: usize = 4_000_000;
}

impl visionfive2_sd::SleepOps for SleepHelper {
    fn sleep_ms(ms: usize) {
        let start = unsafe { Self::read_tick() };
        while unsafe { Self::read_tick() } - start < ms * Self::VF2_FREQ / 1000 {
            core::hint::spin_loop();
        }
    }

    fn sleep_ms_until(ms: usize, mut f: impl FnMut() -> bool) {
        let start = unsafe { Self::read_tick() };
        while unsafe { Self::read_tick() } - start < ms * Self::VF2_FREQ / 1000 {
            if f() {
                return;
            }
            core::hint::spin_loop();
        }
    }
}

pub struct VisionFive2Disk {
    sector: usize,
    offset: usize,
    driver: Vf2SdDriver<VisionFive2SdMMIO, SleepHelper>,
}

impl VisionFive2Disk {
    const SECTOR_SIZE: usize = 512;

    pub fn new(hal: VisionFive2SdMMIO) -> VisionFive2Disk {
        let driver = Vf2SdDriver::<_, SleepHelper>::new(hal);
        VisionFive2Disk {
            sector: 0,
            offset: 0,
            driver,
        }
    }
}

impl IRawDiskDevice for VisionFive2Disk {
    fn read_blocks(&mut self, buf: &mut [u8]) {
        debug_assert_eq!(buf.len(), 512);
        self.driver.read_block(self.sector, buf);
    }

    fn write_blocks(&mut self, buf: &[u8]) {
        debug_assert_eq!(buf.len(), 512);
        self.driver.write_block(self.sector, buf);
    }

    fn get_position(&self) -> usize {
        self.sector * Self::SECTOR_SIZE + self.offset
    }

    fn set_position(&mut self, position: usize) {
        self.sector = position / Self::SECTOR_SIZE;
        self.offset = position % Self::SECTOR_SIZE;
    }
}
