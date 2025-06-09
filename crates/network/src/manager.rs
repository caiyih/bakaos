use core::{cell::UnsafeCell, ops::Deref};

use alloc::{boxed::Box, collections::btree_map::BTreeMap, format, sync::Arc, vec::Vec};
use hermit_sync::SpinMutex;
use network_abstractions::{INetDevice, RxBuffer, SocketType};
use smoltcp::{
    iface::{Config, Interface, SocketHandle, SocketSet},
    phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken},
    socket::tcp,
    wire::HardwareAddress,
};

use crate::socket::SocketFile;

pub(crate) struct DeviceWrapper {
    inner: Arc<SpinMutex<Box<dyn INetDevice>>>,
}

unsafe impl Send for DeviceWrapper {}
unsafe impl Sync for DeviceWrapper {}

impl Deref for DeviceWrapper {
    type Target = Arc<SpinMutex<Box<dyn INetDevice>>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub(crate) struct CustomRxToken(Arc<SpinMutex<Box<dyn INetDevice>>>, RxBuffer);
pub(crate) struct CustomTxToken(Arc<SpinMutex<Box<dyn INetDevice>>>);

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
        _timestamp: smoltcp::time::Instant,
    ) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        match self.lock().receive() {
            Ok(buf) => Some((
                CustomRxToken(self.inner.clone(), buf),
                CustomTxToken(self.inner.clone()),
            )),
            // TODO: handle types of errors
            Err(()) => None,
        }
    }

    fn transmit(&mut self, _timestamp: smoltcp::time::Instant) -> Option<Self::TxToken<'_>> {
        Some(CustomTxToken(self.inner.clone()))
    }

    fn capabilities(&self) -> smoltcp::phy::DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1536;
        caps.max_burst_size = Some(1);
        caps.medium = Medium::Ethernet;
        caps
    }
}

pub struct NetworkManager {
    device: UnsafeCell<DeviceWrapper>,
    state: Arc<SpinMutex<NetworkState>>,
}

impl alloc::fmt::Debug for NetworkManager {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> alloc::fmt::Result {
        f.debug_struct("NetworkManager").finish()
    }
}

impl NetworkManager {
    pub(crate) fn new(device: Box<dyn INetDevice>, seed: Option<u64>) -> Self {
        let mut config = Config::new();

        config.hardware_addr = Some(HardwareAddress::Ethernet(smoltcp::wire::EthernetAddress(
            device.mac_address().mac,
        )));

        if let Some(seed) = seed {
            config.random_seed = seed;
        }

        let mut device = DeviceWrapper {
            inner: Arc::new(SpinMutex::new(device)),
        };

        let interface = Interface::new(config, &mut device);

        let sockets = SocketSet::new(Vec::new());

        Self {
            device: UnsafeCell::new(device),
            state: Arc::new(SpinMutex::new(NetworkState {
                interface: UnsafeCell::new(interface),
                sockets: UnsafeCell::new(sockets),
                ports: BTreeMap::new(),
                socket_threshold: 16,
                socket_count: 0,
            })),
        }
    }
}

impl NetworkManager {
    pub(crate) fn device_mut(&self) -> &mut DeviceWrapper {
        unsafe { self.device.get().as_mut().unwrap() }
    }

    pub fn state(&self) -> &SpinMutex<NetworkState> {
        &self.state
    }

    pub fn create_socket(&self, socket_type: SocketType, port: u16) -> Option<SocketFile> {
        const RX_BUF_LEN: usize = 1024;
        const TX_BUF_LEN: usize = 1024;

        use alloc::vec;

        assert!(socket_type == SocketType::Tcp);

        let mut state = self.state.lock();

        if state.socket_threshold > state.socket_count + 1 {
            if !state.ports.contains_key(&port) {
                state.socket_count += 1;

                let rx_buf = vec![0; RX_BUF_LEN];
                let tx_buf = vec![0; TX_BUF_LEN];

                let mut tcp_socket = tcp::Socket::new(
                    tcp::SocketBuffer::new(rx_buf),
                    tcp::SocketBuffer::new(tx_buf),
                );

                tcp_socket.listen(port).unwrap();

                let handle = state.sockets_mut().add(tcp_socket);

                state.ports.insert(port, (handle, socket_type)).unwrap();

                return Some(SocketFile {
                    port,
                    handle,
                    name: format!("{:?}Socket(port: {})", socket_type, port),
                    mgr: self.state.clone(),
                    socket_type,
                });
            }
        }
        None
    }
}

pub struct NetworkState {
    interface: UnsafeCell<Interface>,
    sockets: UnsafeCell<SocketSet<'static>>,
    ports: BTreeMap<u16, (SocketHandle, SocketType)>,
    socket_threshold: i16,
    socket_count: i16,
}

impl NetworkState {
    pub fn ifac(&self) -> &Interface {
        unsafe { self.interface.get().as_ref().unwrap() }
    }

    pub fn ifac_mut(&self) -> &mut Interface {
        unsafe { self.interface.get().as_mut().unwrap() }
    }

    pub fn sockets(&self) -> &SocketSet<'static> {
        unsafe { self.sockets.get().as_ref().unwrap() }
    }

    pub fn sockets_mut(&self) -> &mut SocketSet<'static> {
        unsafe { self.sockets.get().as_mut().unwrap() }
    }

    pub fn ports(&self) -> &BTreeMap<u16, (SocketHandle, SocketType)> {
        &self.ports
    }

    pub fn ports_mut(&mut self) -> &mut BTreeMap<u16, (SocketHandle, SocketType)> {
        &mut self.ports
    }
}
