mod baremetal;
mod hosted;

#[cfg(feature = "boot")]
mod entry;

#[cfg(feature = "boot")]
pub use entry::*;
