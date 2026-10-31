use crate::driver::AsyncIO;
use crate::net::utils::bind;
use std::io::{Error, Result};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::{fmt, future};

pub struct UdpSocket(AsyncIO<mio::net::UdpSocket>);

impl UdpSocket {
    fn new(socket: mio::net::UdpSocket) -> Result<UdpSocket> {
        Ok(UdpSocket(AsyncIO::new(socket)?))
    }

    fn bind_addr(addr: SocketAddr) -> Result<UdpSocket> {
        UdpSocket::new(mio::net::UdpSocket::bind(addr)?)
    }

    pub fn bind<A>(addr: A) -> impl Future<Output = Result<Self>>
    where
        A: ToSocketAddrs,
    {
        future::ready(bind(addr, UdpSocket::bind_addr))
    }

    pub fn connect<A>(&self, addr: A) -> impl Future<Output = Result<()>> + use<'_, A>
    where
        A: ToSocketAddrs,
    {
        future::ready(bind(addr, |addr| self.0.io.connect(addr)))
    }

    pub fn send<'b>(&mut self, buf: &'b [u8]) -> impl Future<Output = Result<usize>> + use<'_, 'b> {
        self.0.io_write(|io| io.send(buf))
    }

    pub fn recv<'b>(
        &mut self,
        buf: &'b mut [u8],
    ) -> impl Future<Output = Result<usize>> + use<'_, 'b> {
        self.0.io_read(|io| io.recv(buf))
    }

    pub fn send_to<'b>(
        &mut self,
        buf: &'b [u8],
        target: SocketAddr,
    ) -> impl Future<Output = Result<usize>> + use<'_, 'b> {
        self.0.io_write(move |io| io.send_to(buf, target))
    }

    pub fn recv_from<'b>(
        &mut self,
        buf: &'b mut [u8],
    ) -> impl Future<Output = Result<(usize, SocketAddr)>> + use<'_, 'b> {
        self.0.io_read(|io| io.recv_from(buf))
    }

    pub fn peek_from<'b>(
        &mut self,
        buf: &'b mut [u8],
    ) -> impl Future<Output = Result<(usize, SocketAddr)>> + use<'_, 'b> {
        self.0.io_read(|io| io.peek_from(buf))
    }

    pub fn peek<'b>(
        &mut self,
        buf: &'b mut [u8],
    ) -> impl Future<Output = Result<usize>> + use<'_, 'b> {
        self.0.io_read(|io| io.peek(buf))
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.0.io.local_addr()
    }

    pub fn peer_addr(&self) -> Result<SocketAddr> {
        self.0.io.peer_addr()
    }

    pub fn broadcast(&self) -> Result<bool> {
        self.0.io.broadcast()
    }

    pub fn set_broadcast(&self, on: bool) -> Result<()> {
        self.0.io.set_broadcast(on)
    }

    pub fn multicast_loop_v4(&self) -> Result<bool> {
        self.0.io.multicast_loop_v4()
    }

    pub fn set_multicast_loop_v4(&self, on: bool) -> Result<()> {
        self.0.io.set_multicast_loop_v4(on)
    }

    pub fn multicast_ttl_v4(&self) -> Result<u32> {
        self.0.io.multicast_ttl_v4()
    }

    pub fn set_multicast_ttl_v4(&self, ttl: u32) -> Result<()> {
        self.0.io.set_multicast_ttl_v4(ttl)
    }

    pub fn multicast_loop_v6(&self) -> Result<bool> {
        self.0.io.multicast_loop_v6()
    }

    pub fn set_multicast_loop_v6(&self, on: bool) -> Result<()> {
        self.0.io.set_multicast_loop_v6(on)
    }

    pub fn ttl(&self) -> Result<u32> {
        self.0.io.ttl()
    }

    pub fn set_ttl(&self, ttl: u32) -> Result<()> {
        self.0.io.set_ttl(ttl)
    }

    pub fn join_multicast_v4(&self, multiaddr: Ipv4Addr, interface: Ipv4Addr) -> Result<()> {
        self.0.io.join_multicast_v4(&multiaddr, &interface)
    }

    pub fn join_multicast_v6(&self, multiaddr: &Ipv6Addr, interface: u32) -> Result<()> {
        self.0.io.join_multicast_v6(multiaddr, interface)
    }

    pub fn leave_multicast_v4(&self, multiaddr: Ipv4Addr, interface: Ipv4Addr) -> Result<()> {
        self.0.io.leave_multicast_v4(&multiaddr, &interface)
    }

    pub fn leave_multicast_v6(&self, multiaddr: &Ipv6Addr, interface: u32) -> Result<()> {
        self.0.io.leave_multicast_v6(multiaddr, interface)
    }

    pub fn take_error(&self) -> Result<Option<Error>> {
        self.0.io.take_error()
    }
}

impl fmt::Debug for UdpSocket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}
