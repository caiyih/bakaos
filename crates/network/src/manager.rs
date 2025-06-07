use core::ops::Deref;

use alloc::{boxed::Box, sync::Arc};
use hermit_sync::SpinMutex;
use network_abstractions::{INetDevice, RxBuffer};
use smoltcp::{
    iface::{Config, Interface},
    phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken},
    wire::HardwareAddress,
};

struct NetworkManager {
    device: Box<dyn INetDevice>,
    config: Config,
    interface: Interface,
}

struct DeviceWrapper {
    inner: Arc<SpinMutex<Box<dyn INetDevice>>>,
}

impl Deref for DeviceWrapper {
    type Target = Arc<SpinMutex<Box<dyn INetDevice>>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

struct CustomRxToken(Arc<SpinMutex<Box<dyn INetDevice>>>, RxBuffer);
struct CustomTxToken(Arc<SpinMutex<Box<dyn INetDevice>>>);

impl RxToken for CustomRxToken {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut rx_buf = self.1;
        let result = f(rx_buf.packet_mut());
        self.0.lock().recycle_rx_buffer(rx_buf).unwrap();
        result
    }
}

impl TxToken for CustomTxToken {
    fn consume<R, F>(self, len: usize, func: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut dev = self.0.lock();
        let mut tx_buf = dev.new_tx_buffer(len);
        let result = func(&mut tx_buf);

        dev.send(tx_buf).unwrap();
        result
    }
}

impl Device for DeviceWrapper {
    type RxToken<'a>
        = CustomRxToken
    where
        Self: 'a;

    type TxToken<'a>
        = CustomTxToken
    where
        Self: 'a;

    fn receive(
        &mut self,
        timestamp: smoltcp::time::Instant,
    ) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        todo!()
    }

    fn transmit(&mut self, timestamp: smoltcp::time::Instant) -> Option<Self::TxToken<'_>> {
        todo!()
    }

    fn capabilities(&self) -> smoltcp::phy::DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1536;
        caps.max_burst_size = Some(1);
        caps.medium = Medium::Ethernet;
        caps
    }
}

impl NetworkManager {
    pub fn new(device: Box<dyn INetDevice>) -> Self {
        let mut config = Config::new();

        config.hardware_addr = Some(HardwareAddress::Ethernet(smoltcp::wire::EthernetAddress(
            device.mac_address().mac,
        )));

        let mut device = DeviceWrapper {
            inner: Arc::new(SpinMutex::new(device)),
        };

        let interface = Interface::new(config, &mut device);

        unimplemented!()
    }
}
