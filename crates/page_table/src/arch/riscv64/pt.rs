use abstractions::IUsizeAlias;
use address::{PhysicalAddress, VirtualAddress};

use crate::pt::IPageTableArchAttribute;

pub struct SV39PageTableAttribute;

impl IPageTableArchAttribute for SV39PageTableAttribute {
    const LEVELS: usize = 3;
    const PA_MAX_BITS: usize = 56;
    const VA_MAX_BITS: usize = 39;

    fn flush_tlb(vaddr: address::VirtualAddress) {
        unsafe {
            if vaddr == VirtualAddress::from_usize(0) {
                riscv::asm::sfence_vma_all();
            } else {
                riscv::asm::sfence_vma(0, vaddr.as_usize())
            }
        }
    }

    #[inline(always)]
    fn is_higher_half_activated(_paddr: PhysicalAddress) -> bool {
        true
    }

    #[inline(always)]
    fn is_lower_half_activated(paddr: PhysicalAddress) -> bool {
        #[cfg(target_arch = "riscv64")]
        {
            phys_to_satp(paddr) == platform_specific::satp()
        }

        #[cfg(not(target_arch = "riscv64"))]
        {
            true
        }
    }

    fn activate(paddr: PhysicalAddress, lazy_flush: bool) {
        let satp = phys_to_satp(paddr);

        if lazy_flush {
            unsafe {
                ::core::arch::asm!(
                    "
                        csrw satp, {0}
                    ", 
                    in(reg) satp
                );
            }
        } else {
            unsafe {
                ::core::arch::asm!(
                    "
                        csrw satp, {0}
                        sfence.vma
                    ", 
                    in(reg) satp
                );
            }
        }
    }
}

#[inline(always)]
fn phys_to_satp(paddr: PhysicalAddress) -> usize {
    (paddr.as_usize() >> 12) | (8 << 60)
}
