use core::arch::global_asm;

use loongArch64::{
    self,
    register::{
        ecfg::{self},
        eentry, euen, pgdh, pgdl, pwch, pwcl, stlbps, tcfg, tlbidx, tlbrehi, tlbrentry,
    },
};
use platform_specific::virt_to_phys;

use crate::{clear_bss, loongarch64::context::init_thread_info};

#[naked]
#[no_mangle]
#[link_section = ".text.entry"] // Don't rename, cross crates inter-operation
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn _start() -> ! {
    ::core::arch::naked_asm!(
        "
            # Configure DMW0. VSEG = 8, PLV0, Strongly ordered uncachd
            li.d        $t0, (0x8000000000000000 | 1)
            csrwr       $t0, 0x180

            # Configure DMW1. VSEG = 9, PLV0, Coherent cached
            li.d        $t0, (0x9000000000000000 | 1 | 1 << 4)
            csrwr       $t0, 0x181

            # Setup stack for main thread
            la.global   $sp, __tmp_stack_top

            # Initialize virtual memory
            bl          {init_boot_page_table}

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

            bl          {init_mmu}          # setup boot page table and enabel MMU
            invtlb      0x00, $r0, $r0

            # Enable PG 
            li.w		$t0, 0xb0		# PLV=0, IE=0, PG=1
            csrwr		$t0, 0x0        # LOONGARCH_CSR_CRMD
            li.w		$t0, 0x00		# PLV=0, PIE=0, PWE=0
            csrwr		$t0, 0x1        # LOONGARCH_CSR_PRMD
            li.w		$t0, 0x00		# FPE=0, SXE=0, ASXE=0, BTE=0
            csrwr		$t0, 0x2        # LOONGARCH_CSR_EUEN

            # aka. u0 in Linux
            csrrd       $r21, 0x20           # cpuid

            bl          {main_processor_init}

            la.global   $t0, __kernel_start_main

            # We can't use bl to jump to higher address, so we use jirl to jump to higher address.
            jirl        $zero, $t0, 0
            ",
        init_boot_page_table = sym init_boot_page_table,
        init_mmu = sym init_mmu,
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

unsafe extern "C" fn init_mmu() {
    // Page Size 4KB
    const PS_4K: usize = 0x0c;
    tlbidx::set_ps(PS_4K);
    stlbps::set_ps(PS_4K);
    tlbrehi::set_ps(PS_4K);

    let paddr = virt_to_phys(&raw const PT_L0 as usize);
    pgdh::set_base(paddr);
    pgdl::set_base(0);
}

// Huge Page Mapping Flags: V | D | HUGE | P | W
const HUGE_FLAGS: u64 = (1 << 0) | (1 << 1) | (1 << 6) | (1 << 7) | (1 << 8);

#[link_section = ".data.prepage"]
static mut PT_L0: [u64; 512] = [0; 512];

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

unsafe extern "C" fn init_boot_page_table() {
    let l1_va = &raw const PT_L1 as usize;
    let l1_pa = virt_to_phys(l1_va) as u64;

    // 0x0000_0000_0000 ~ 0x0080_0000_0000 identity mapping
    // but we are access using higher half address space, so accessing with an offsest of  0xffff_0000_0000_0000
    // See LoongArch64 reference manual 5.4.5 and 7.5.6 for more info
    PT_L0[0] = l1_pa;
}

extern "C" fn main_processor_init() {
    // loongson,ls7a-rtc
    // https://github.com/qemu/qemu/blob/661c2e1ab29cd9c4d268ae3f44712e8d421c0e56/include/hw/pci-host/ls7a.h#L45
    const RTC_BASE: usize = 0x10000000 + 0x00080000 + 0x00050100;
    const SYS_RTCCTRL: usize = 0x40;

    const RTC_MASK: u64 = ((!0u64) >> (64 - (1))) << (13);
    const TOY_MASK: u64 = ((!0u64) >> (64 - (1))) << (11);
    const EO_MASK: u64 = ((!0u64) >> (64 - (1))) << (8);

    let rtc_ctrl = ((RTC_BASE + SYS_RTCCTRL) | 0x8000_0000_0000_0000) as *mut u32;
    unsafe {
        rtc_ctrl.write_volatile((TOY_MASK | EO_MASK | RTC_MASK) as u32);
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
}

fn set_trap_vector_base(eentry: usize) {
    ecfg::set_vs(0);
    eentry::set_eentry(eentry);
}
