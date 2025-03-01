mod context;
mod registers;
mod serial;
pub mod syscall_ids;

use core::ffi::CStr;

pub(crate) use context::TaskTrapContext;
pub use registers::*;
pub use serial::*;

pub const PLATFORM_STRING: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"RISC-V64\0") };

pub const VIRT_ADDR_OFFSET: usize = 0xffff_ffc0_0000_0000;
pub const PHYS_ADDR_MASK: usize = 0x0000_003f_ffff_ffff;

#[allow(unused)]
pub unsafe fn get_instruction_size(pc: usize) -> Result<usize, usize> {
    // https://stackoverflow.com/questions/56874101/how-does-risc-v-variable-length-of-instruction-work-in-detail?rq=3

    let p_ins16 = pc as *const u16;
    let ins_header = p_ins16.read_unaligned();

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

    Err(pc)
}

#[allow(unused)]
pub unsafe fn find_previous_instruction(ra: usize) -> Result<usize, usize> {
    fn to<T>(ra: usize) -> *const T {
        (ra - core::mem::size_of::<T>()) as *const T
    }

    let p_ins32 = to::<u32>(ra);
    let ins32 = p_ins32.read_unaligned();

    // Fast path for 'unimp' instruction
    if ins32 == 0 {
        return Ok(p_ins32 as usize);
    }

    // Is a 32bit instruction
    if ins32 & 0b11 == 0b11 && ins32 & 0b11111 != 0b11111 {
        return Ok(p_ins32 as usize);
    }

    let p_ins16 = to::<u16>(ra);
    let ins16 = p_ins16.read_unaligned();

    // 16bit instructio starts with 0x00, 0x01 or 0x10
    if ins16 & 0b11 != 0b11 {
        return Ok(p_ins16 as usize);
    }

    let p_ins64 = to::<u64>(ra);
    let ins64 = p_ins64.read_unaligned();

    // 64bits instruction
    if ins64 & 0b1111111 == 0b0111111 {
        return Ok(p_ins64 as usize);
    }

    let p_ins48 = to::<u16>(ra).offset(-2);
    let ins48_header = p_ins48.read_unaligned();

    if ins48_header & 0b111111 == 0b011111 {
        return Ok(p_ins48 as usize);
    }

    Err(ra)
}

#[inline]
pub fn current_processor_index() -> usize {
    tp()
}
