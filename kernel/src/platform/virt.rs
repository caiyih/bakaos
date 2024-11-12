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
}
