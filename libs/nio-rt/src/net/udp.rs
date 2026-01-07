use crate::driver::AsyncIO;
use crate::net::utils::bind;
use std::io::Result;
use std::net::{SocketAddr, ToSocketAddrs};
use std::ops::Deref;
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

    pub fn send<'b>(&self, buf: &'b [u8]) -> impl Future<Output = Result<usize>> + use<'_, 'b> {
        self.0.async_io_write(|io| io.send(buf))
    }

    pub fn recv<'b>(&self, buf: &'b mut [u8]) -> impl Future<Output = Result<usize>> + use<'_, 'b> {
        self.0.async_io_read(|io| io.recv(buf))
    }

    pub fn send_to<'b>(
        &self,
        buf: &'b [u8],
        target: SocketAddr,
    ) -> impl Future<Output = Result<usize>> + use<'_, 'b> {
        self.0.async_io_write(move |io| io.send_to(buf, target))
    }

    pub fn recv_from<'b>(
        &self,
        buf: &'b mut [u8],
    ) -> impl Future<Output = Result<(usize, SocketAddr)>> + use<'_, 'b> {
        self.0.async_io_read(|io| io.recv_from(buf))
    }

    pub fn peek_from<'b>(
        &self,
        buf: &'b mut [u8],
    ) -> impl Future<Output = Result<(usize, SocketAddr)>> + use<'_, 'b> {
        self.0.async_io_read(|io| io.peek_from(buf))
    }
}

impl Deref for UdpSocket {
    type Target = mio::net::UdpSocket;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0.io
    }
}

impl fmt::Debug for UdpSocket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}
