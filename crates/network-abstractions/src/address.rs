use core::fmt::Debug;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct EthernetAddress {
    pub mac: [u8; 6],
}

impl Debug for EthernetAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.mac[0], self.mac[1], self.mac[2], self.mac[3], self.mac[4], self.mac[5]
        )
    }
}
