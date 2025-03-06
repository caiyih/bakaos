#![no_std]
#![feature(panic_can_unwind)]

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

impl<const N: usize> StackTrace<N> {
    #[inline(always)]
    #[allow(unused)]
    pub fn begin_unwind(skip_frames: usize) -> StackTrace<N> {
        #[cfg(not(any(target_arch = "riscv64", target_arch = "loongarch64")))]
        panic!("Unsupported architecture");

        #[cfg(target_arch = "loongarch64")]
        {
            Self::loongarch64_begin_unwind(skip_frames)
        }

        #[cfg(target_arch = "riscv64")]
        {
            Self::riscv64_begin_unwind(skip_frames)
        }
    }

    pub fn stack_frames(&self) -> &[StackFrame] {
        &self.frames.as_slice()[..self.len]
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
impl<const N: usize> StackTrace<N> {
    #[inline(always)]
    fn riscv64_begin_unwind(mut skip_frames: usize) -> StackTrace<N> {
        extern "C" {
            fn stext();
            fn etext();
        }

        let mut ra = platform_specific::ra();
        let mut fp = platform_specific::fp();

        let mut traces = unsafe { core::mem::zeroed::<StackTrace<N>>() };
        let mut len = 0;

        while len <= N
            && ra >= stext as usize
            && ra <= etext as usize
            && fp >= stext as usize
            && fp != 0
        {
            if skip_frames == 0 {
                let pc = unsafe { platform_specific::find_previous_instruction(ra) };
                traces.frames[len] = StackFrame { pc, fp };
                len += 1
            } else {
                skip_frames -= 1;
            }

            ra = unsafe { *(fp as *const usize).offset(-1) };
            fp = unsafe { *(fp as *const usize).offset(-2) };
        }

        traces.len = len;

        traces
    }
}

#[cfg(target_arch = "loongarch64")]
impl<const N: usize> StackTrace<N> {
    #[inline(always)]
    fn loongarch64_begin_unwind(mut skip_frames: usize) -> StackTrace<N> {
        extern "C" {
            fn stext();
            fn etext();
        }

        let mut ra = platform_specific::ra();
        let mut fp = platform_specific::fp();

        let mut traces = unsafe { core::mem::zeroed::<StackTrace<N>>() };
        let mut len = 0;

        while len <= N
            && ra >= stext as usize
            && ra <= etext as usize
            && fp >= stext as usize
            && fp != 0
        {
            if skip_frames == 0 {
                let pc = ra - 4; // all instructions on loongarch64 are 32-bit
                traces.frames[len] = StackFrame { pc: Ok(pc), fp };
                len += 1
            } else {
                skip_frames -= 1;
            }

            ra = unsafe { *(fp as *const usize).offset(-1) };
            fp = unsafe { *(fp as *const usize).offset(-2) };
        }

        traces.len = len;

        traces
    }
}
