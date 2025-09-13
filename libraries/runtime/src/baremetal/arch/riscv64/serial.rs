use alloc_crate::fmt;
use hermit_sync::SpinMutex;

pub static SERIAL_PORT: SpinMutex<WriteOnlySerialPort> = SpinMutex::new(WriteOnlySerialPort);

pub struct WriteOnlySerialPort;

impl fmt::Write for WriteOnlySerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            if byte == b'\n' {
                console_putchar(b'\r');
            }

            console_putchar(byte);
        }

        Ok(())
    }
}

#[inline(always)]
fn console_putchar(c: u8) {
    // TODO: error handling
    let _ = sbi_rt::console_write_byte(c);
}
