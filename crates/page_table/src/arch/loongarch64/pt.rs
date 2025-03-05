use abstractions::IUsizeAlias;
use address::{IAddressBase, PhysicalAddress, VirtualAddress};

use crate::IPageTableArchAttribute;

pub struct LA64PageTableAttribute;

impl IPageTableArchAttribute for LA64PageTableAttribute {
    const LEVELS: usize = 4;
    const PA_MAX_BITS: usize = 48;
    const VA_MAX_BITS: usize = 48;

    fn flush_tlb(vaddr: VirtualAddress) {
        unsafe {
            if vaddr.is_null() {
                // op 0x0: Clear all page table entries
                ::core::arch::asm!("dbar 0; invtlb 0x00, $r0, $r0");
            } else {
                // <https://loongson.github.io/LoongArch-Documentation/LoongArch-Vol1-EN.html#_dbar>
                //
                // Only after all previous load/store access operations are completely
                // executed, the DBAR 0 instruction can be executed; and only after the
                // execution of DBAR 0 is completed, all subsequent load/store access
                // operations can be executed.
                //
                // <https://loongson.github.io/LoongArch-Documentation/LoongArch-Vol1-EN.html#_invtlb>
                //
                // formats: invtlb op, asid, addr
                //
                // op 0x5: Clear all page table entries with G=0 and ASID equal to the
                // register specified ASID, and VA equal to the register specified VA.
                //
                // When the operation indicated by op does not require an ASID, the
                // general register rj should be set to r0.
                ::core::arch::asm!("dbar 0; invtlb 0x05, $r0, {reg}", reg = in(reg) vaddr.as_usize());
            }
        }
    }

    fn is_higher_half_activated(_paddr: PhysicalAddress) -> bool {
        true
    }

    fn is_lower_half_activated(paddr: PhysicalAddress) -> bool {
        platform_specific::pgdl() == paddr.as_usize()
    }

    fn activate(paddr: PhysicalAddress, _lazy_flush: bool) {
        // TODO: temporarily disable lazy flush
        unsafe {
            ::core::arch::asm!(
                "
                    csrwr {0}, 0x19
                    dbar 0
                    invtlb 0x00, $r0, $r0
                ",
                in(reg) paddr.as_usize()
            );
        }
    }

    fn activated_table() -> PhysicalAddress {
        PhysicalAddress::from_usize(platform_specific::pgdl())
    }
}
