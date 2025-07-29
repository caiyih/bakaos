#[cfg(feature = "virt")]
mod virt;

#[cfg(feature = "2k1000")]
mod _2k1000;

#[cfg(feature = "virt")]
pub use virt::boot_init;

#[cfg(feature = "2k1000")]
pub use _2k1000::boot_init;
