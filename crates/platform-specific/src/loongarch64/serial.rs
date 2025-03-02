const UART_BASE: usize = 0x1FE001E0;

// 0x8000_XXXX_XXXX_XXXX is uncached, which is good for mmio access.
static UART: Uart = Uart::new(UART_BASE | 0x8000_0000_0000_0000);

#[no_mangle]
#[allow(static_mut_refs)]
pub fn console_writestr(str: &[u8]) {
    for &c in str {
        console_putchar(c);
    }
}

#[no_mangle]
#[inline(always)]
#[allow(static_mut_refs)]
pub fn console_putchar(c: u8) {
    UART.putchar(c);
}

#[no_mangle]
#[inline(always)]
#[allow(static_mut_refs)]
pub fn console_getchar() -> Option<u8> {
    UART.getchar()
}

// adapted from https://github.com/Byte-OS/polyhal/blob/main/src/components/debug_console/loongarch64.rs
struct Uart {
    base_address: usize,
}

impl Uart {
    pub const fn new(base_address: usize) -> Self {
        Uart { base_address }
    }

    pub fn putchar(&self, c: u8) {
        let ptr = self.base_address as *mut u8;
        loop {
            unsafe {
                if ptr.add(5).read_volatile() & (1 << 5) != 0 {
                    break;
                }
            }
        }
        unsafe {
            ptr.add(0).write_volatile(c);
        }
    }

    pub fn getchar(&self) -> Option<u8> {
        let ptr = self.base_address as *mut u8;
        unsafe {
            if ptr.add(5).read_volatile() & 1 == 0 {
                // The DR bit is 0, meaning no data
                None
            } else {
                // The DR bit is 1, meaning data!
                Some(ptr.add(0).read_volatile())
            }
        }
    }
}
