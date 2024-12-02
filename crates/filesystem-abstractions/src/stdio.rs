use crate::{FileDescriptorBuilder, FrozenFileDescriptorBuilder, ICacheableFile};
use alloc::sync::Arc;

use crate::IFile;

pub struct TeleTypewriterBuilder {
    pub stdin_builder: FrozenFileDescriptorBuilder,
    pub stdout_builder: FrozenFileDescriptorBuilder,
    pub stderr_builder: FrozenFileDescriptorBuilder,
}

impl TeleTypewriterBuilder {
    pub fn open_for(task_id: usize) -> Self {
        let tty: Arc<dyn IFile> = Arc::new(TeleTypewriter { task_id });

        let stdin_accessor = tty.cache_as_arc_accessor();
        let stdout_accessor = stdin_accessor.clone_non_inherited_arc();
        let stderr_accessor = stdin_accessor.clone_non_inherited_arc();

        Self {
            stdin_builder: FileDescriptorBuilder::new(stdin_accessor)
                .set_readable()
                .freeze(),
            stdout_builder: FileDescriptorBuilder::new(stdout_accessor)
                .set_writable()
                .freeze(),
            stderr_builder: FileDescriptorBuilder::new(stderr_accessor)
                .set_writable()
                .freeze(),
        }
    }
}

pub trait IStdioFile: IFile {
    fn task_id(&self) -> usize;
}

#[derive(Debug)]
struct TeleTypewriter {
    task_id: usize,
}

impl IStdioFile for TeleTypewriter {
    fn task_id(&self) -> usize {
        self.task_id
    }
}

impl IFile for TeleTypewriter {
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
