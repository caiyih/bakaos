use alloc::sync::Arc;

use crate::IFile;

pub trait IStdioFile: IFile {
    fn open_for(task_id: usize) -> Arc<dyn IFile>;

    fn task_id(&self) -> usize;
}

#[derive(Debug)]
pub struct Stdin {
    task_id: usize,
}

impl IStdioFile for Stdin {
    fn open_for(task_id: usize) -> Arc<dyn IFile> {
        Arc::new(Stdin { task_id })
    }

    fn task_id(&self) -> usize {
        self.task_id
    }
}

impl IFile for Stdin {
    fn metadata(&self) -> Option<Arc<crate::FileMetadata>> {
        None
    }

    fn lseek(&self, _offset: usize) -> usize {
        0
    }

    fn can_read(&self) -> bool {
        true
    }

    fn read(&self, buf: &mut [u8]) -> usize {
        // TODO: Extract this into separate crate
        fn getchar_from_serial() -> Option<u8> {
            #[cfg(not(target_arch = "riscv64"))]
            panic!("getchar_from_serial not implemented for this target");

            #[cfg(target_arch = "riscv64")]
            {
                let mut ch: i8;
                unsafe {
                    core::arch::asm!(
                        "ecall",
                        in("a7") 0x02, // legacy getchar eid
                        lateout("a0") ch,
                    );
                }

                match ch {
                    -1 => None,
                    _ => Some(ch as u8),
                }
            }
        }

        let mut read_bytes = 0;

        while let Some(ch) = getchar_from_serial() {
            buf[read_bytes] = ch;
            read_bytes += 1;

            if read_bytes >= buf.len() {
                break;
            }
        }

        read_bytes
    }
}

pub struct Stdout {
    task_id: usize,
}

impl IStdioFile for Stdout {
    fn open_for(task_id: usize) -> Arc<dyn IFile> {
        Arc::new(Stdout { task_id })
    }

    fn task_id(&self) -> usize {
        self.task_id
    }
}

impl IFile for Stdout {
    fn metadata(&self) -> Option<Arc<crate::FileMetadata>> {
        None
    }

    fn lseek(&self, _offset: usize) -> usize {
        0
    }

    fn can_write(&self) -> bool {
        true
    }

    fn write(&self, buf: &[u8]) -> usize {
        #[allow(unused_variables)]
        fn putchar_to_serial(ch: u8) {
            #[cfg(not(target_arch = "riscv64"))]
            panic!("putchar_to_serial not implemented for this target");

            #[cfg(target_arch = "riscv64")]
            unsafe {
                core::arch::asm!(
                    "ecall",
                    in("a7") 0x01, // legacy putchar eid
                    in("a0") ch,
                );
            }
        }

        let mut written_bytes = 0;

        for &ch in buf {
            // TODO: Figure out whether we should break on null byte
            if ch == 0 {
                break;
            }

            putchar_to_serial(ch);
            written_bytes += 1;
        }

        written_bytes
    }
}

// Since Stderr also prints to serial, we just alias it to Stdout
pub type Stderr = Stdout;
