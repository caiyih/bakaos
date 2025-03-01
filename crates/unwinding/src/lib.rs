#![no_std]
#![feature(panic_can_unwind)]

extern crate alloc;

use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct StackTrace {
    frames: Vec<StackFrame>,
}

#[derive(Debug, Clone, Copy)]
pub struct StackFrame {
    pc: Result<usize, usize>,
    fp: usize,
}

impl StackTrace {
    #[inline(always)]
    #[allow(unused)]
    pub fn begin_unwind(skip_frames: usize) -> StackTrace {
        #[cfg(not(any(target_arch = "riscv64")))]
        panic!("Unsupported architecture");

        #[cfg(target_arch = "riscv64")]
        Self::riscv64_begin_unwind(skip_frames)
    }

    pub fn stack_frames(&self) -> &[StackFrame] {
        self.frames.as_slice()
    }
}

impl StackFrame {
    #[inline(always)]
    pub fn fp(&self) -> usize {
        self.fp
    }

    #[inline(always)]
    pub fn pc(&self) -> Result<usize, usize> {
        self.pc
    }
}

impl StackTrace {
    #[cfg(target_arch = "riscv64")]
    #[inline(always)]
    fn riscv64_begin_unwind(mut skip_frames: usize) -> StackTrace {
        extern "C" {
            fn stext();
            fn etext();
        }

        let mut ra = platform_specific::ra();
        let mut fp = platform_specific::fp();
        let mut frames = Vec::new();

        while ra >= stext as usize && ra <= etext as usize && fp >= stext as usize && fp != 0 {
            if skip_frames == 0 {
                let pc = unsafe { platform_specific::find_previous_instruction(ra) };
                frames.push(StackFrame { pc, fp })
            } else {
                skip_frames -= 1;
            }

            fp = unsafe { *(fp as *const usize).offset(-2) };
            ra = unsafe { *(fp as *const usize).offset(-1) };
        }

        StackTrace { frames }
    }
}
