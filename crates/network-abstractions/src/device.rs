use crate::{address::EthernetAddress, buffer::{RxBuffer, TxBuffer}};

pub type Result<T> = core::result::Result<T, ()>; // FIXME: add error type

pub trait INetDevice {
    fn mac_address(&self) -> EthernetAddress;

    fn can_send(&self) -> bool;

    fn can_recv(&self) -> bool;

    fn receive(&mut self) -> Result<RxBuffer>;

    fn recycle_rx_buffer(&mut self, rx_buf: RxBuffer) -> Result<()>;

    fn new_tx_buffer(&self, buf_len: usize) -> TxBuffer;
 
    fn send(&mut self, tx_buf: TxBuffer) -> Result<()>;
}
