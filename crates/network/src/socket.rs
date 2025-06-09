use core::fmt::Debug;

use alloc::{string::String, sync::Arc};
use filesystem_abstractions::{IFile, OpenFlags};
use hermit_sync::SpinMutex;
use network_abstractions::SocketType;
use smoltcp::{
    iface::SocketHandle,
    socket::{tcp, udp},
};

use crate::manager::NetworkState;

const IO_CHUNK: usize = 1460;

pub struct SocketFile {
    pub(crate) port: u16,
    pub(crate) handle: SocketHandle,
    pub(crate) name: String,
    pub(crate) mgr: Arc<SpinMutex<NetworkState>>,
    pub(crate) socket_type: SocketType,
}

impl Debug for SocketFile {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> alloc::fmt::Result {
        f.debug_struct("SocketFile")
            .field("port", &self.port)
            .field("handle", &self.handle)
            .field("name", &self.name)
            .field("socket_type", &self.socket_type)
            .finish()
    }
}

impl SocketFile {
    fn with_tcp_socket<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut tcp::Socket) -> R,
    {
        let state = self.mgr.lock();

        let socket = state.sockets_mut().get_mut::<tcp::Socket>(self.handle);

        f(socket)
    }

    fn with_udp_socket<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut udp::Socket) -> R,
    {
        let state = self.mgr.lock();

        let socket = state.sockets_mut().get_mut::<udp::Socket>(self.handle);

        f(socket)
    }
}

impl IFile for SocketFile {
    fn metadata(&self) -> Option<Arc<filesystem_abstractions::FileMetadata>> {
        None
    }

    fn can_read(&self) -> bool {
        assert!(self.socket_type == SocketType::Tcp);

        self.with_tcp_socket(|sock| sock.may_recv())
    }

    fn can_write(&self) -> bool {
        assert!(self.socket_type == SocketType::Tcp);

        self.with_tcp_socket(|sock| sock.may_send())
    }

    fn read_avaliable(&self) -> bool {
        assert!(self.socket_type == SocketType::Tcp);

        self.with_tcp_socket(|sock| sock.can_recv())
    }

    fn write_avaliable(&self) -> bool {
        // TODO: check this
        true
    }

    fn flags(&self) -> filesystem_abstractions::OpenFlags {
        // TODO: check this
        OpenFlags::NONE
    }

    fn set_flags(&self, _new_flags: filesystem_abstractions::OpenFlags) -> bool {
        false
    }

    fn inode(&self) -> Option<Arc<filesystem_abstractions::DirectoryTreeNode>> {
        None
    }

    fn is_dir(&self) -> bool {
        false
    }

    fn write(&self, buf: &[u8]) -> usize {
        assert!(self.socket_type == SocketType::Tcp);

        self.with_tcp_socket(|sock| {
            assert!(sock.may_send());

            if !sock.can_send() {
                return 0;
            }

            let want = core::cmp::min(buf.len(), core::cmp::min(IO_CHUNK, sock.send_capacity()));
            sock.send_slice(&buf[..want]).unwrap_or(0)
        })
    }

    fn read(&self, buf: &mut [u8]) -> usize {
        assert!(self.socket_type == SocketType::Tcp);

        self.with_tcp_socket(|sock| {
            assert!(sock.may_recv());

            if !sock.can_recv() {
                return 0;
            }

            let want = core::cmp::min(buf.len(), IO_CHUNK);

            sock.recv(|data| {
                let cnt = core::cmp::min(want, data.len());
                buf[..cnt].copy_from_slice(&data[..cnt]);
                (cnt, cnt)
            })
            .unwrap_or(0)
        })
    }

    fn pread(&self, buf: &mut [u8], _offset: u64) -> usize {
        self.read(buf)
    }

    fn pwrite(&self, buf: &[u8], _offset: u64) -> usize {
        self.write(buf)
    }
}

impl Drop for SocketFile {
    fn drop(&mut self) {
        if self.socket_type == SocketType::Tcp {
            self.with_tcp_socket(|socket| {
                if socket.state() == tcp::State::Established {
                    socket.close();
                }
            });
        } else if self.socket_type == SocketType::Udp {
            self.with_udp_socket(|socket| {
                socket.close();
            });
        }

        let mut mgr = self.mgr.lock();

        mgr.ports_mut().remove(&self.port);

        mgr.sockets_mut().remove(self.handle);
    }
}
