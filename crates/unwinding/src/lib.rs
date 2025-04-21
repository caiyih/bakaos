#![no_std]
#![feature(trait_alias)]
#![feature(panic_can_unwind)]

use core::ops::Deref;

#[derive(Debug, Clone)]
pub struct StackTrace<const N: usize> {
    frames: [StackFrame; N],
    len: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct StackFrame {
    pc: Result<usize, usize>,
    fp: usize,
}

pub trait BacktraceCallbackDelegate = FnMut(usize, StackFrame) -> bool + Sized;

pub struct StackTraceWalker;

impl StackTraceWalker {
    #[inline(always)]
    #[allow(unused)]
    pub fn begin_unwind(skip_frames: usize, callback: impl BacktraceCallbackDelegate) {
        #[cfg(not(any(target_arch = "riscv64", target_arch = "loongarch64")))]
        panic!("Unsupported architecture");

        #[cfg(target_arch = "loongarch64")]
        {
            Self::loongarch64_begin_unwind(skip_frames, callback)
        }

        #[cfg(target_arch = "riscv64")]
        {
            Self::riscv64_begin_unwind(skip_frames, callback)
        }
    }
}

impl<const N: usize> StackTrace<N> {
    #[inline(always)]
    #[allow(unused)]
    pub fn collect(skip_frames: usize) -> StackTrace<N> {
        let mut traces = unsafe { core::mem::zeroed::<StackTrace<N>>() };

        let mut len = 0;

        StackTraceWalker::begin_unwind(skip_frames, |index, frame| {
            traces.frames[index] = frame;
            len += 1;

            len < N
        });

        traces.len = len;

        traces
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<const N: usize> Deref for StackTrace<N> {
    type Target = [StackFrame];

    fn deref(&self) -> &Self::Target {
        assert!(self.len <= N);

        &self.frames
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

#[cfg(target_arch = "riscv64")]
impl StackTraceWalker {
    #[inline(always)]
    fn riscv64_begin_unwind(mut skip_frames: usize, mut callback: impl BacktraceCallbackDelegate) {
        extern "C" {
            fn stext();
            fn etext();
        }

        let mut ra = platform_specific::ra();
        let mut fp = platform_specific::fp();

        let mut index = 0;

        while ra >= stext as usize && ra <= etext as usize && fp >= stext as usize && fp != 0 {
            if skip_frames == 0 {
                let pc = unsafe { platform_specific::find_previous_instruction(ra) };

                if !callback(index, StackFrame { pc, fp }) {
                    break;
                }

                index += 1;
            } else {
                skip_frames -= 1;
            }

            ra = unsafe { *(fp as *const usize).offset(-1) };
            fp = unsafe { *(fp as *const usize).offset(-2) };
        }
    }
}

#[cfg(target_arch = "loongarch64")]
impl StackTraceWalker {
    #[inline(always)]
    fn loongarch64_begin_unwind(
        mut skip_frames: usize,
        mut callback: impl BacktraceCallbackDelegate,
    ) {
        extern "C" {
            fn stext();
            fn etext();
        }

        let mut ra = platform_specific::ra();
        let mut fp = platform_specific::fp();

        let mut index = 0;

        while ra >= stext as usize && ra <= etext as usize && fp >= stext as usize && fp != 0 {
            if skip_frames == 0 {
                let pc = ra - 4; // all instructions on loongarch64 are 32-bit

                if !callback(index, StackFrame { pc: Ok(pc), fp }) {
                    break;
                }

                index += 1;
            } else {
                skip_frames -= 1;
            }

            ra = unsafe { *(fp as *const usize).offset(-1) };
            fp = unsafe { *(fp as *const usize).offset(-2) };
        }
    }
}
