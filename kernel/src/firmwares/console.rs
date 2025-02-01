use core::{
    arch::asm,
    cmp,
    fmt::{self, Write},
};

use hermit_sync::SpinMutex;

const BUFFER_CAPACITY: usize = 4096;

struct RingBuffer {
    buffer: [u8; BUFFER_CAPACITY],
    head: usize,
    tail: usize,
    len: usize,
}

static DMESG_BUFFER: SpinMutex<RingBuffer> = SpinMutex::new(RingBuffer {
    buffer: [0; BUFFER_CAPACITY],
    head: 0,
    tail: 0,
    len: 0,
});

pub fn read_dmesg(buffer: &mut [u8]) -> usize {
    let dmesg = DMESG_BUFFER.lock();
    let read_len = cmp::min(buffer.len(), dmesg.len);

    for (i, ch) in buffer.iter_mut().enumerate().take(read_len) {
        *ch = dmesg.buffer[(dmesg.head + i) % BUFFER_CAPACITY];
    }

    read_len
}

fn push_message(msg: &str) {
    let mut dmesg = DMESG_BUFFER.lock();
    let msg_bytes = msg.as_bytes();
    let msg_len = msg_bytes.len();

    if msg_len > BUFFER_CAPACITY {
        let start = msg_len - BUFFER_CAPACITY;
        dmesg.buffer.copy_from_slice(&msg_bytes[start..]);
        dmesg.head = 0;
        dmesg.tail = 0;
        dmesg.len = BUFFER_CAPACITY;
        return;
    }

    while dmesg.len + msg_len > BUFFER_CAPACITY {
        dmesg.head = (dmesg.head + 1) % BUFFER_CAPACITY;
        dmesg.len -= 1;
    }

    for &b in msg_bytes {
        let tail = dmesg.tail;
        dmesg.buffer[tail] = b;
        dmesg.tail = (tail + 1) % BUFFER_CAPACITY;
    }

    dmesg.len += msg_len;

    debug_assert!(dmesg.len <= BUFFER_CAPACITY);
}

pub trait IConsole: Write {
    #[allow(unused)]
    fn put_char(&self, c: u8) -> fmt::Result;

    #[allow(unused)]
    fn get_char(&self) -> u8;

    #[allow(unused)]
    fn name(&self) -> &'static str;
}

#[derive(Clone, Copy)]
pub struct LegacyConsole;

impl LegacyConsole {
    #[allow(unused)]
    fn get_api() -> Self {
        Self
    }
}

const LEGACY_PUTCHAR_EID: usize = 0x01;
const LEGACY_GETCHAR_EID: usize = 0x02;

impl Write for LegacyConsole {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.bytes() {
            self.put_char(c)?;
        }

        push_message(s);

        Ok(())
    }
}

impl LegacyConsole {
    #[allow(unused)]
    fn write_fmt(&mut self, args: fmt::Arguments) {
        Write::write_fmt(self, args).unwrap();
    }
}

impl IConsole for LegacyConsole {
    fn put_char(&self, c: u8) -> fmt::Result {
        unsafe {
            asm!(
                "ecall",
                in("a0") c as usize,
                in("a7") LEGACY_PUTCHAR_EID,
            );
        }
        Ok(())
    }

    fn get_char(&self) -> u8 {
        let mut ret: u8;

        unsafe {
            asm!(
                "ecall",
                lateout("a0") ret,
                in("a7") LEGACY_GETCHAR_EID,
            );
        }

        ret
    }

    fn name(&self) -> &'static str {
        "LegacyConsole"
    }
}
