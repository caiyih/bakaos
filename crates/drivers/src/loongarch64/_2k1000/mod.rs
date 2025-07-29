use timing::TimeSpec;

use crate::IMachine;

#[allow(unused)]
pub fn machine_2k1000() -> &'static dyn IMachine {
    static INSTANCE: LS2K1000Machine = LS2K1000Machine;
    &INSTANCE
}

struct LS2K1000Machine;

impl IMachine for LS2K1000Machine {
    fn name(&self) -> &'static str {
        "Loongson 2K1000"
    }

    fn query_performance_frequency(&self) -> u64 {
        // Calculaetd with code below.
        // Since this is a constant for every machine, we can hardcode it.
        // let cc_freq = loongArch64::cpu::CPUCFG::read(0x4).get_bits(0, 31) as u64;
        // let cc_mul = loongArch64::cpu::CPUCFG::read(0x5).get_bits(0, 15) as u64;
        // let cc_div = loongArch64::cpu::CPUCFG::read(0x5).get_bits(16, 31) as u64;

        // cc_freq * cc_mul / cc_div

        100000000
    }

    fn memory_end(&self) -> usize {
        0x0000000090000000 + 0x0000000070000000
    }

    fn mmio(&self) -> &[(usize, usize)] {
        &[]
    }

    fn query_performance_counter(&self) -> usize {
        platform_specific::stable_counter()
    }

    fn create_block_device_at(
        &self,
        _device_id: usize,
    ) -> alloc::sync::Arc<crate::BlockDeviceInode> {
        todo!("Block device")
    }

    fn get_rtc_offset(&self) -> timing::TimeSpec {
        TimeSpec::zero()
    }
}
