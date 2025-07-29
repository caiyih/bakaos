use core::arch::global_asm;

use loongArch64::{
    self,
    register::{
        ecfg::{self},
        eentry, euen, tcfg,
    },
};

use crate::{clear_bss, loongarch64::context::init_thread_info};

#[naked]
#[no_mangle]
#[link_section = ".text.entry"] // Don't rename, cross crates inter-operation
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn _start() -> ! {
    ::core::arch::naked_asm!(
        "
            .macro SET_CSR_BITS CSR_ID, bits_start, bits_end, value
                csrrd      $t0, \\CSR_ID

                li.d       $t2, (1 << (\\bits_end - \\bits_start + 1)) - 1
                slli.d     $t2, $t2, \\bits_start

                and        $t0, $t0, $t2

                li.d       $t1, \\value
                slli.d     $t1, $t1, \\bits_start

                # for safety consideration
                and        $t0, $t0, $t2

                or         $t0, $t0, $t1

                csrwr      $t0, \\CSR_ID
            .endm
        
            # Configure DMW0. VSEG = 8, PLV0, Strongly ordered uncachd
            li.d        $t0, (0x8000000000000000 | 1)
            csrwr       $t0, 0x180

            # Configure DMW1. VSEG = 9, PLV0, Coherent cached
            li.d        $t0, (0x9000000000000000 | 1 | 1 << 4)
            csrwr       $t0, 0x181

            # Setup stack for main thread
            la.global   $sp, __tmp_stack_top

            li.d        $t2, 0x0000ffffffffffff # PHYS_ADDR_MASK

            la.global   $t0, PT_L0
            la.global   $t1, PT_L1
            and         $t1, $t1, $t2
            st.d        $t1, $t0, 0     # PT_L0[0] = phys(PT_L1)

            # 5. Configure MMU

            # PTE width: 0 for 8 bytes
            # 512 entries for each levels of dir/pt
            # | PTE width | PT base | PT width | Dir1_base    | Dir1_width | Dir2_base      | Dir2_width |
            # | 0 << 30   | 12      | 9 << 5   | (12+9) << 10 | 9 << 15    | (12+9+9) << 20 | 9 << 25    |
            li.d        $t0, ((0 << 30) | 12 | (9 << 5) | ((12+9)<<10) | (9<<15) | ((12+9*2)<<20) | (9<<25))
            csrwr       $t0, 0x1c       # LOONGARCH_CSR_PWCL
            # | Dir3_base | dir3_width |
            li.d        $t0, ((12+9*3) | (9<<6))
            csrwr       $t0, 0x1d       # LOONGARCH_CSR_PWCH

            # 2. Setup temporary tlb refill exception handler
            li.d        $t2, 0x0000ffffffffffff # PHYS_ADDR_MASK

            # According to loongarch reference manual, this must be a physical address and 4k aligned
            la.global   $t0, handle_tlb_refill
            and         $t0, $t0, $t2
            csrwr       $t0, 0x88       # LOONGARCH_CSR_TLBRENTRY

            SET_CSR_BITS 0x10, 24, 29, 12 # TLBIDX
            SET_CSR_BITS 0x1e,  0,  5, 12 # STLBPS
            SET_CSR_BITS 0x8e,  0,  5, 12 # TLBREHI

            invtlb      0x00, $r0, $r0

            # Enable PG for current mode
            li.w		$t0, 1 << 4
            csrwr		$t0, 0x0        # LOONGARCH_CSR_CRMD

            # aka. u0 in Linux
            csrrd       $r21, 0x20           # cpuid
            
            move        $a0, $r21
            bl          {main_processor_init}

            la.global   $t0, __kernel_start_main

            # We can't use bl to jump to higher address, so we use jirl to jump to higher address.
            jirl        $zero, $t0, 0
            ",
        main_processor_init = sym main_processor_init,
    )
}

global_asm!(
    "
.section .text
.balign 4096
.global handle_tlb_refill
handle_tlb_refill:
         csrwr   $t0, 0x8b               # LA_CSR_TLBRSAVE, KScratch for TLB refill exception
         csrrd   $t0, 0x1b               # LA_CSR_PGD, Page table base
         lddir   $t0, $t0, 3
         lddir   $t0, $t0, 2
         lddir   $t0, $t0, 1
         ldpte   $t0, 0
         ldpte   $t0, 1
         tlbfill
         csrrd   $t0, 0x8b               # LA_CSR_TLBRSAVE
         ertn
"
);

// Huge Page Mapping Flags: V | D | HUGE | P | W
const HUGE_FLAGS: u64 = (1 << 0) | (1 << 1) | (1 << 6) | (1 << 7) | (1 << 8);

#[no_mangle]
#[link_section = ".data.prepage"]
static mut PT_L0: [u64; 512] = [0; 512];

#[no_mangle]
#[link_section = ".data.prepage"]
static mut PT_L1: [u64; 512] = {
    let mut pt_l1 = [0; 512];
    // 0x0000_0000..0x4000_0000, VRWX_GAD, 1G block
    pt_l1[0] = HUGE_FLAGS;
    // 0x4000_0000..0x8000_0000, VRWX_GAD, 1G block
    pt_l1[1] = 0x4000_0000 | HUGE_FLAGS;
    // 0x8000_0000..0xc000_0000, VRWX_GAD, 1G block
    pt_l1[2] = 0x8000_0000 | HUGE_FLAGS;
    pt_l1
};

extern "C" fn main_processor_init(r21: usize) {
    if r21 != 0 {
        platform_specific::legacy_println!("Non main CPU({}) booting, go sleeping", r21);

        loop {
            unsafe {
                core::arch::asm!("idle 0");
            }
        }
    }

    unsafe { clear_bss() };

    // Enable floating point
    euen::set_fpe(true);

    unsafe { init_thread_info() };

    extern "C" {
        fn trap_vector_base();
    }

    set_trap_vector_base(trap_vector_base as usize);

    tcfg::set_init_val(0);
    tcfg::set_periodic(false);
    tcfg::set_en(true);

    platform_specific::boot_init();
}

fn set_trap_vector_base(eentry: usize) {
    ecfg::set_vs(0);
    eentry::set_eentry(eentry);
}
