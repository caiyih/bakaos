use core::marker::PhantomData;

use address::{IAddressBase, VirtualAddress};

use crate::IPageTableArchAttribute;

#[derive(Debug)]
pub struct FlushHandle<Arch: IPageTableArchAttribute> {
    vaddr: VirtualAddress,
    _marker: PhantomData<Arch>,
}

impl<Arch: IPageTableArchAttribute> FlushHandle<Arch> {
    pub fn new(vaddr: VirtualAddress) -> FlushHandle<Arch> {
        FlushHandle {
            vaddr,
            _marker: PhantomData,
        }
    }

    pub fn all() -> FlushHandle<Arch> {
        FlushHandle {
            vaddr: VirtualAddress::null(),
            _marker: PhantomData,
        }
    }

    pub fn flush(self) {
        Arch::flush_tlb(self.vaddr)
    }

    pub fn vaddr(&self) -> VirtualAddress {
        self.vaddr
    }

    pub fn is_flush_all(&self) -> bool {
        self.vaddr.is_null()
    }
}
