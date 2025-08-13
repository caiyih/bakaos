use alloc::sync::Arc;
use kernel_abstractions::IKernelSerial;

pub(crate) struct KernelSerial;

impl KernelSerial {
    pub fn new() -> Arc<KernelSerial> {
        Arc::new(Self {})
    }
}

impl IKernelSerial for KernelSerial {
    fn send(&self, byte: u8) -> Result<(), &'static str> {
        platform_specific::console_putchar(byte);

        Ok(())
    }

    fn recv(&self) -> Option<u8> {
        platform_specific::console_getchar()
    }
}
