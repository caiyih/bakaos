#![cfg_attr(not(feature = "std"), no_std)]

use core::mem::MaybeUninit;

use hermit_sync::SpinMutex;
use rand_chacha::{
    rand_core::{RngCore, SeedableRng},
    ChaCha20Rng,
};
use rand_xoshiro::Xoshiro256StarStar;

#[cfg(feature = "std")]
extern crate std;

static GLOBAL_XOSHIRO256SS: SpinMutex<MaybeUninit<Xoshiro256StarStar>> =
    SpinMutex::new(MaybeUninit::uninit());
static GLOBAL_CHACHA20: SpinMutex<MaybeUninit<ChaCha20Rng>> = SpinMutex::new(MaybeUninit::uninit());

pub fn global_next64_fast() -> u64 {
    unsafe { GLOBAL_XOSHIRO256SS.lock().assume_init_mut().next_u64() }
}

pub fn global_next32_fast() -> u32 {
    unsafe { GLOBAL_XOSHIRO256SS.lock().assume_init_mut().next_u32() }
}

pub fn global_fill_fast(buffer: &mut [u8]) {
    unsafe {
        GLOBAL_XOSHIRO256SS
            .lock()
            .assume_init_mut()
            .fill_bytes(buffer);
    }
}

pub fn global_next64_safe() -> u64 {
    unsafe { GLOBAL_CHACHA20.lock().assume_init_mut().next_u64() }
}

pub fn global_next32_safe() -> u32 {
    unsafe { GLOBAL_CHACHA20.lock().assume_init_mut().next_u32() }
}

pub fn global_fill_safe(buffer: &mut [u8]) {
    unsafe {
        GLOBAL_CHACHA20.lock().assume_init_mut().fill_bytes(buffer);
    }
}

pub fn initialize(seed: u64) {
    *GLOBAL_XOSHIRO256SS.lock() = MaybeUninit::new(Xoshiro256StarStar::seed_from_u64(seed));
    *GLOBAL_CHACHA20.lock() = MaybeUninit::new(ChaCha20Rng::seed_from_u64(seed));
}
