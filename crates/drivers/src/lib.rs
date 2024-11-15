#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

pub mod virt;
pub use virt::*;

pub trait IDiskDevice: Sync + Send {
    fn read_blocks(&mut self, buf: &mut [u8]);

    fn write_blocks(&mut self, buf: &[u8]);

    fn get_position(&self) -> usize;

    fn set_position(&mut self, position: usize);

    fn move_cursor(&mut self, amount: usize);
}
