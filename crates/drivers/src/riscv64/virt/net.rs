use network_abstractions::INetDevice;
use virtio_drivers::{
    device::net::VirtIONet,
    transport::{mmio::MmioTransport, Transport},
    Hal,
};

const NET_QUEUE_SIZE: usize = 16;
const NET_BUFFER_LEN: usize = 2048;

struct VirtioNetDevice<H: Hal, T: Transport> {
    device: VirtIONet<H, T, NET_QUEUE_SIZE>,
}

impl<H: Hal, T: Transport> VirtioNetDevice<H, T> {
    pub fn new(transport: T) -> Self {
        VirtioNetDevice {
            device: VirtIONet::new(transport, NET_BUFFER_LEN).unwrap(),
        }
    }
}

impl<H: Hal, T: Transport> INetDevice for VirtioNetDevice<H, T> {
    fn mac_address(&self) -> network_abstractions::EthernetAddress {
        network_abstractions::EthernetAddress {
            mac: self.device.mac_address()
        }
    }

    fn can_send(&self) -> bool {
        self.device.can_send()
    }

    fn can_recv(&self) -> bool {
        self.device.can_recv()
    }

    fn receive(&mut self) -> network_abstractions::Result<network_abstractions::RxBuffer> {
        todo!()
    }

    fn recycle_rx_buffer(&mut self, rx_buf: network_abstractions::RxBuffer) -> network_abstractions::Result<()> {
        todo!()
    }

    fn new_tx_buffer(&self, buf_len: usize) -> network_abstractions::TxBuffer {
        todo!()
    }

    fn send(&mut self, tx_buf: network_abstractions::TxBuffer) -> network_abstractions::Result<()> {
        todo!()
    }
}
