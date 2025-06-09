#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod manager;
mod socket;

use alloc::boxed::Box;
use hermit_sync::OnceCell;
pub use manager::*;
use network_abstractions::INetDevice;
use smoltcp::time::Instant;
pub use socket::SocketFile;

static MANAGER: OnceCell<NetworkManager> = OnceCell::new();

pub fn init(device: Box<dyn INetDevice>, seed: Option<u64>) {
    let manager = NetworkManager::new(device, seed);

    MANAGER
        .set(manager)
        .expect("Can not set network manager twice");
}

pub fn manager() -> &'static NetworkManager {
    MANAGER.get().expect("Network manager is not initialized")
}

pub fn poll(timestamp: i64) {
    let manager = manager();
    let state = manager.state().lock();

    state.ifac_mut().poll(
        Instant::from_micros(timestamp),
        manager.device_mut(),
        state.sockets_mut(),
    );
}
