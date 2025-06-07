use alloc::boxed::Box;

use network_abstractions::INetDevice;

struct NetworkManager {
    device: Box<dyn INetDevice>,
}
