#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct StackTrace {
    frames: Vec<StackFrame>,
}

#[derive(Debug, Clone, Copy)]
pub struct StackFrame {
    ra: usize,
    fp: usize,
}

#[inline(always)]
pub fn sp() -> usize {
    #[cfg(not(target_arch = "riscv64"))]
    panic!("Unsupported architecture");

    #[cfg(target_arch = "riscv64")]
    {
        let ptr;
        unsafe {
            core::arch::asm!("mv {}, sp", out(reg) ptr);
        }
        ptr
    }
}

#[inline(always)]
pub fn fp() -> usize {
    #[cfg(not(target_arch = "riscv64"))]
    panic!("Unsupported architecture");

    #[cfg(target_arch = "riscv64")]
    {
        let ptr;
        unsafe {
            core::arch::asm!("mv {}, fp", out(reg) ptr);
        }
        ptr
    }
}

#[inline(always)]
pub fn lr() -> usize {
    #[cfg(not(target_arch = "riscv64"))]
    panic!("Unsupported architecture");

    #[cfg(target_arch = "riscv64")]
    {
        let ptr;
        unsafe {
            core::arch::asm!("mv {}, ra", out(reg) ptr);
        }
        ptr
    }
}

impl StackTrace {
    #[inline(always)]
    #[allow(unused)]
    pub fn begin_unwind(skip_frames: usize) -> StackTrace {
        #[cfg(not(target_arch = "riscv64"))]
        panic!("Unsupported architecture");

        #[cfg(target_arch = "riscv64")]
        Self::riscv64_begin_unwind(skip_frames)
    }

    pub fn stack_frames(&self) -> &[StackFrame] {
        self.frames.as_slice()
    }

    #[cfg(target_arch = "riscv64")]
    #[inline(always)]
    fn riscv64_begin_unwind(mut skip_frames: usize) -> StackTrace {
        extern "C" {
            fn stext();
            fn etext();
        }

        let mut ra = lr();
        let mut fp = fp();
        let mut frames = Vec::new();

        while ra >= stext as usize && ra <= etext as usize && fp >= stext as usize && fp != 0 {
            if skip_frames == 0 {
                frames.push(StackFrame { ra, fp })
            } else {
                skip_frames -= 1;
            }

            fp = unsafe { *(fp as *const usize).offset(-2) };
            ra = unsafe { *(fp as *const usize).offset(-1) };
        }

        StackTrace { frames }
    }
}

impl StackFrame {
    #[inline(always)]
    pub fn fp(&self) -> usize {
        self.fp
    }

    #[inline(always)]
    pub fn ra(&self) -> usize {
        self.ra
    }
}

#[allow(unused)]
pub fn find_previous_instruction(ra: usize) -> Result<usize, u64> {
    #[cfg(not(target_arch = "riscv64"))]
    panic!("Unsupported architecture");

    #[cfg(target_arch = "riscv64")]
    unsafe {
        riscv64_ra_to_pc(ra)
    }
}

#[allow(unused)]
pub fn get_instruction_size(pc: usize) -> Result<usize, u64> {
    #[cfg(not(target_arch = "riscv64"))]
    panic!("Unsupported architecture");

    #[cfg(target_arch = "riscv64")]
    unsafe {
        riscv64_get_instruction_size(pc)
    }
}

#[allow(unused)]
unsafe fn riscv64_get_instruction_size(pc: usize) -> Result<usize, u64> {
    // https://stackoverflow.com/questions/56874101/how-does-risc-v-variable-length-of-instruction-work-in-detail?rq=3

    let p_ins16 = pc as *const u16;
    let ins_header = p_ins16.read();

    // 32bits instruction
    if ins_header & 0b11 == 0b11 && ins_header & 0b11111 != 0b11111 {
        return Ok(4);
    }

    // 16bits instruction
    if ins_header & 0b11 != 0b11 {
        return Ok(1);
    }

    // 64bits instruction
    if ins_header & 0b1111111 == 0b0111111 {
        return Ok(8);
    }

    // 48bits instruction
    if ins_header & 0b111111 == 0b011111 {
        return Ok(6);
    }

    Err((pc as *const u64).read())
}

#[allow(unused)]
unsafe fn riscv64_ra_to_pc(ra: usize) -> Result<usize, u64> {
    fn to<T>(ra: usize) -> *const T {
        (ra - core::mem::size_of::<T>()) as *const T
    }

    let p_ins32 = to::<u32>(ra);
    let ins32 = p_ins32.read();

    // Fast path for 'unimp' instruction
    if ins32 == 0 {
        return Ok(p_ins32 as usize);
    }

    // Is a 32bit instruction
    if ins32 & 0b11 == 0b11 && ins32 & 0b11111 != 0b11111 {
        return Ok(p_ins32 as usize);
    }

    let p_ins16 = to::<u16>(ra);
    let ins16 = p_ins16.read();

    // 16bit instructio starts with 0x00, 0x01 or 0x10
    if ins16 & 0b11 != 0b11 {
        return Ok(p_ins16 as usize);
    }

    let p_ins64 = to::<u64>(ra);
    let ins64 = p_ins64.read();

    // 64bits instruction
    if ins64 & 0b1111111 == 0b0111111 {
        return Ok(p_ins64 as usize);
    }

    let p_ins48 = to::<u16>(ra).offset(-2);
    let ins48_header = p_ins48.read();

    if ins48_header & 0b111111 == 0b011111 {
        return Ok(p_ins48 as usize);
    }

    Err(ins64)
}
