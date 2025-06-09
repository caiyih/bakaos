use core::ops::{Deref, DerefMut};

use alloc::{vec, vec::Vec};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

pub type RxBuffer = ReciveBuffer;
pub type TxBuffer = TransmitBuffer;

#[repr(C)]
#[derive(FromBytes, KnownLayout, Immutable)]
pub struct NetHeader {
    flags: u8,
    gso_type: u8,
    hdr_len: u16, // cannot rely on this
    gso_size: u16,
    csum_start: u16,
    csum_offset: u16,
}

const HEADER_SIZE: usize = core::mem::size_of::<NetHeader>();

pub struct ReciveBuffer {
    pub(crate) buf: Vec<usize>, // for alignment
    pub(crate) packet_len: usize,
    pub(crate) idx: u16,
}

impl ReciveBuffer {
    /// Allocates a new buffer with length `buf_len`.
    pub fn new(idx: usize, buf_len: usize) -> Self {
        Self {
            buf: vec![0; buf_len / size_of::<usize>()],
            packet_len: 0,
            idx: idx.try_into().unwrap(),
        }
    }

    /// Set the network packet length.
    pub fn set_packet_len(&mut self, packet_len: usize) {
        self.packet_len = packet_len
    }

    /// Returns the network packet length (witout header).
    pub const fn packet_len(&self) -> usize {
        self.packet_len
    }

    /// Returns all data in the buffer, including both the header and the packet.
    pub fn as_bytes(&self) -> &[u8] {
        self.buf.as_bytes()
    }

    /// Returns all data in the buffer with the mutable reference,
    /// including both the header and the packet.
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.buf.as_mut_bytes()
    }

    // Returns the reference of the header.
    pub fn header(&self) -> &NetHeader {
        FromBytes::ref_from_prefix(self.as_bytes()).unwrap().0
    }

    /// Returns the network packet as a slice.
    pub fn packet(&self) -> &[u8] {
        &self.buf.as_bytes()[HEADER_SIZE..HEADER_SIZE + self.packet_len]
    }

    /// Returns the network packet as a mutable slice.
    pub fn packet_mut(&mut self) -> &mut [u8] {
        &mut self.buf.as_mut_bytes()[HEADER_SIZE..HEADER_SIZE + self.packet_len]
    }

    pub fn idx(&self) -> usize {
        self.idx as usize
    }

    pub fn set_idx(&mut self, idx: usize) {
        self.idx = idx.try_into().unwrap()
    }
}

pub struct TransmitBuffer {
    pub(crate) buf: Vec<u8>,
}

impl TransmitBuffer {
    pub fn new(buf: Vec<u8>) -> Self {
        TransmitBuffer { buf }
    }

    /// Returns the network packet length.
    pub fn packet_len(&self) -> usize {
        self.buf.len()
    }

    /// Returns the network packet as a slice.
    pub fn packet(&self) -> &[u8] {
        &self.buf
    }
}

impl Deref for TransmitBuffer {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.buf
    }
}

impl DerefMut for TransmitBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buf
    }
}
