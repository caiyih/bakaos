#![cfg_attr(not(feature = "std"), no_std)]

use address::{IAlignableAddress, PhysicalAddress, VirtualAddress, VirtualAddressRange};

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod flags;

pub use flags::GenericMappingFlags;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MMUError {
    InvalidAddress,
    PrivilegeError,
    AccessFault, // not mapped to a proper frame
    MisalignedAddress,
    PageNotReadable { vaddr: VirtualAddress },
    PageNotWritable { vaddr: VirtualAddress },
}

impl dyn IPageTable {
    pub fn access<T>(&self, vaddr: VirtualAddress) -> Result<&T, MMUError> {
        ensure_aligned::<T>(vaddr)?;

        unsafe {
            self.inspect_bytes(vaddr, core::mem::size_of::<T>())
                .map(|_| vaddr.as_ptr::<T>().as_ref().unwrap())
        }
    }

    pub fn access_mut<T>(&self, vaddr: VirtualAddress) -> Result<&mut T, MMUError> {
        ensure_aligned::<T>(vaddr)?;

        unsafe {
            self.inspect_bytes_mut(vaddr, core::mem::size_of::<T>())
                .map(|_| vaddr.as_mut_ptr::<T>().as_mut().unwrap())
        }
    }

    pub fn access_slice<T>(&self, vaddr: VirtualAddress, len: usize) -> Result<&[T], MMUError> {
        ensure_aligned::<T>(vaddr)?;

        unsafe {
            self.inspect_bytes(vaddr, len * core::mem::size_of::<T>())
                .map(|_| core::slice::from_raw_parts(vaddr.as_ptr::<T>(), len))
        }
    }

    pub fn access_slice_mut<T>(
        &self,
        vaddr: VirtualAddress,
        len: usize,
    ) -> Result<&mut [T], MMUError> {
        ensure_aligned::<T>(vaddr)?;

        unsafe {
            self.inspect_bytes_mut(vaddr, len * core::mem::size_of::<T>())
                .map(|_| core::slice::from_raw_parts_mut(vaddr.as_mut_ptr::<T>(), len))
        }
    }

    pub fn import<T: Copy>(&self, vaddr: VirtualAddress) -> Result<T, MMUError> {
        let mut value: T = unsafe { core::mem::zeroed() };
        let value_bytes = unsafe {
            core::slice::from_raw_parts_mut(
                &mut value as *mut T as *mut u8,
                core::mem::size_of::<T>(),
            )
        };

        self.read_bytes(vaddr, value_bytes).map(|_| value)
    }

    pub fn export<T: Copy>(&self, vaddr: VirtualAddress, value: T) -> Result<(), MMUError> {
        let value_bytes = unsafe {
            core::slice::from_raw_parts(&value as *const T as *const u8, core::mem::size_of::<T>())
        };

        self.write_bytes(vaddr, value_bytes)
    }

    #[cfg(not(target_os = "none"))]
    pub fn register<T>(&mut self, val: &T, mutable: bool) -> VirtualAddress {
        self.register_internal(
            VirtualAddress::from_ref(val),
            core::mem::size_of_val(val),
            mutable,
        );

        VirtualAddress::from_ref(val)
    }

    #[cfg(not(target_os = "none"))]
    pub fn unregister<T>(&mut self, val: &T) {
        self.unregister_internal(VirtualAddress::from_ref(val));
    }
}

fn ensure_aligned<T>(vaddr: VirtualAddress) -> Result<(), MMUError> {
    if !vaddr.is_aligned(core::mem::align_of::<T>()) {
        return Err(MMUError::MisalignedAddress);
    }

    Ok(())
}

pub trait IPageTable {
    fn map_single(
        &mut self,
        vaddr: VirtualAddress,
        target: PhysicalAddress,
        size: PageSize,
        flags: GenericMappingFlags,
    ) -> PagingResult<()>;

    fn remap_single(
        &mut self,
        vaddr: VirtualAddress,
        new_target: PhysicalAddress,
        flags: GenericMappingFlags,
    ) -> PagingResult<PageSize>;

    fn unmap_single(&mut self, vaddr: VirtualAddress) -> PagingResult<(PhysicalAddress, PageSize)>;

    fn query_virtual(
        &self,
        vaddr: VirtualAddress,
    ) -> PagingResult<(PhysicalAddress, GenericMappingFlags, PageSize)>;

    fn create_or_update_single(
        &mut self,
        vaddr: VirtualAddress,
        size: PageSize,
        paddr: Option<PhysicalAddress>,
        flags: Option<GenericMappingFlags>,
    ) -> PagingResult<()>;

    fn translate_continuous(
        &self,
        vaddr: VirtualAddress,
        size: usize,
    ) -> Result<VirtualAddressRange, MMUError>;

    fn translate_page(&self, vaddr: VirtualAddress) -> Result<VirtualAddress, MMUError>;

    fn translate_continuous_paddr(
        &self,
        paddr: PhysicalAddress,
        size: usize,
    ) -> Result<VirtualAddressRange, MMUError>;

    unsafe fn translate_paddr(&self, paddr: PhysicalAddress) -> Result<VirtualAddress, MMUError>;

    fn inspect_bytes(&self, vaddr: VirtualAddress, len: usize) -> Result<&[u8], MMUError>;

    fn inspect_bytes_mut(&self, vaddr: VirtualAddress, len: usize) -> Result<&mut [u8], MMUError>;

    fn read_bytes(&self, vaddr: VirtualAddress, buf: &mut [u8]) -> Result<(), MMUError>;

    fn write_bytes(&self, vaddr: VirtualAddress, buf: &[u8]) -> Result<(), MMUError>;

    fn platform_payload(&self) -> usize;

    #[cfg(not(target_os = "none"))]
    fn register_internal(&mut self, vaddr: VirtualAddress, len: usize, mutable: bool);

    #[cfg(not(target_os = "none"))]
    fn unregister_internal(&mut self, vaddr: VirtualAddress);
}

// impl_downcast!(IPageTable);

/// The error type for page table operation failures.
#[derive(Debug, PartialEq, Eq)]
pub enum PagingError {
    /// The address is not aligned to the page size.
    NotAligned,
    /// The mapping is not present.
    NotMapped,
    /// The mapping is already present.
    AlreadyMapped,
    /// The page table entry represents a huge page, but the target physical
    /// frame is 4K in size.
    MappedToHugePage,
    CanNotModify,
    OutOfMemory,
}

impl Into<MMUError> for PagingError {
    fn into(self) -> MMUError {
        match self {
            PagingError::NotAligned => MMUError::MisalignedAddress,
            PagingError::NotMapped => MMUError::InvalidAddress,
            _ => unimplemented!("Should never happen: {:?}", self),
        }
    }
}

/// The page sizes supported by the hardware page table.
#[repr(usize)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PageSize {
    /// Size of 4 kilobytes (2<sup>12</sup> bytes).
    _4K = 0x1000,
    /// Size of 2 megabytes (2<sup>21</sup> bytes).
    _2M = 0x20_0000,
    /// Size of 1 gigabytes (2<sup>30</sup> bytes).
    _1G = 0x4000_0000,
    Custom(usize),
}

impl From<usize> for PageSize {
    fn from(value: usize) -> Self {
        match value {
            0x1000 => PageSize::_4K,
            0x20_0000 => PageSize::_2M,
            0x4000_0000 => PageSize::_1G,
            _ => PageSize::Custom(value),
        }
    }
}

impl PageSize {
    pub const fn as_usize(&self) -> usize {
        match self {
            PageSize::_4K => 0x1000,
            PageSize::_2M => 0x20_0000,
            PageSize::_1G => 0x4000_0000,
            PageSize::Custom(v) => *v,
        }
    }
}

pub type PagingResult<TValue> = Result<TValue, PagingError>;
