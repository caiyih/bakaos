#![feature(trait_alias)]
#![cfg_attr(not(test), no_std)]

mod invoke_on_drop;

pub use invoke_on_drop::*;
