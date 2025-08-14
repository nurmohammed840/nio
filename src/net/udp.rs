use futures::FutureExt;

use super::utils::bind;
use crate::io::poll_evented::PollEvented;
use std::{
    fmt,
    future::{self, Future},
    io::{self, Result},
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs},
    ops::Deref,
};

pub struct UdpSocket {
    io: PollEvented<mio::net::UdpSocket>,
}

impl UdpSocket {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> impl Future<Output = Result<Self>> {
        future::ready(bind(addr, UdpSocket::bind_addr))
    }

    fn bind_addr(addr: SocketAddr) -> Result<UdpSocket> {
        UdpSocket::new(mio::net::UdpSocket::bind(addr)?)
    }

    fn new(socket: mio::net::UdpSocket) -> Result<UdpSocket> {
        Ok(UdpSocket {
            io: PollEvented::new(socket)?,
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.io.local_addr()
    }

    pub fn peer_addr(&self) -> Result<SocketAddr> {
        self.io.peer_addr()
    }

    pub fn connect<A>(&self, addr: A) -> impl Future<Output = Result<()>> + use<'_, A>
    where
        A: ToSocketAddrs,
    {
        let io = self.io.deref();
        future::ready(bind(addr, |addr| io.connect(addr)))
    }

    pub fn writable(&self) -> impl Future<Output = Result<()>> + '_ {
        self.io.poll_write_readiness().map(|_| Ok(()))
    }

    pub fn send<'b>(&self, buf: &'b [u8]) -> impl Future<Output = Result<usize>> + use<'_, 'b> {
        self.io.async_io_write(|io| io.send(buf))
    }

    pub fn try_send(&self, buf: &[u8]) -> Result<usize> {
        self.io.try_io_write(|io| io.send(buf))
    }

    pub fn readable(&self) -> impl Future<Output = Result<()>> + '_ {
        self.io.poll_read_readiness().map(|_| Ok(()))
    }

    pub fn recv<'b>(&self, buf: &'b mut [u8]) -> impl Future<Output = Result<usize>> + use<'_, 'b> {
        self.io.async_io_read(|io| io.recv(buf))
    }

    pub fn try_recv(&self, buf: &mut [u8]) -> Result<usize> {
        self.io.try_io_read(|io| io.recv(buf))
    }

    pub fn send_to<'b>(
        &self,
        buf: &'b [u8],
        target: SocketAddr,
    ) -> impl Future<Output = Result<usize>> + use<'_, 'b> {
        self.io.async_io_write(move |io| io.send_to(buf, target))
    }

    pub fn try_send_to(&self, buf: &[u8], target: SocketAddr) -> Result<usize> {
        self.io.try_io_write(|io| io.send_to(buf, target))
    }

    pub fn recv_from<'b>(
        &self,
        buf: &'b mut [u8],
    ) -> impl Future<Output = Result<(usize, SocketAddr)>> + use<'_, 'b> {
        self.io.async_io_read(|io| io.recv_from(buf))
    }

    pub fn try_recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        self.io.try_io_read(|io| io.recv_from(buf))
    }

    pub fn peek_from<'b>(
        &self,
        buf: &'b mut [u8],
    ) -> impl Future<Output = Result<(usize, SocketAddr)>> + use<'_, 'b> {
        self.io.async_io_read(|io| io.peek_from(buf))
    }

    pub fn try_peek_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        self.io.try_io_read(|io| io.peek_from(buf))
    }

    pub fn broadcast(&self) -> Result<bool> {
        self.io.broadcast()
    }

    pub fn set_broadcast(&self, on: bool) -> Result<()> {
        self.io.set_broadcast(on)
    }

    pub fn multicast_loop_v4(&self) -> Result<bool> {
        self.io.multicast_loop_v4()
    }

    pub fn set_multicast_loop_v4(&self, on: bool) -> Result<()> {
        self.io.set_multicast_loop_v4(on)
    }

    pub fn multicast_ttl_v4(&self) -> Result<u32> {
        self.io.multicast_ttl_v4()
    }

    pub fn set_multicast_ttl_v4(&self, ttl: u32) -> Result<()> {
        self.io.set_multicast_ttl_v4(ttl)
    }

    pub fn multicast_loop_v6(&self) -> Result<bool> {
        self.io.multicast_loop_v6()
    }

    pub fn set_multicast_loop_v6(&self, on: bool) -> Result<()> {
        self.io.set_multicast_loop_v6(on)
    }

    pub fn ttl(&self) -> Result<u32> {
        self.io.ttl()
    }

    pub fn set_ttl(&self, ttl: u32) -> Result<()> {
        self.io.set_ttl(ttl)
    }

    pub fn join_multicast_v4(&self, multiaddr: Ipv4Addr, interface: Ipv4Addr) -> Result<()> {
        self.io.join_multicast_v4(&multiaddr, &interface)
    }

    pub fn join_multicast_v6(&self, multiaddr: &Ipv6Addr, interface: u32) -> Result<()> {
        self.io.join_multicast_v6(multiaddr, interface)
    }

    pub fn leave_multicast_v4(&self, multiaddr: Ipv4Addr, interface: Ipv4Addr) -> Result<()> {
        self.io.leave_multicast_v4(&multiaddr, &interface)
    }

    pub fn leave_multicast_v6(&self, multiaddr: &Ipv6Addr, interface: u32) -> Result<()> {
        self.io.leave_multicast_v6(multiaddr, interface)
    }

    pub fn take_error(&self) -> Result<Option<io::Error>> {
        self.io.take_error()
    }
}

impl fmt::Debug for UdpSocket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.io.fmt(f)
    }
}
