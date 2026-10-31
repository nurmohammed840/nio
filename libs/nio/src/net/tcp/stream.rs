use crate::driver::AsyncIO;
use crate::net::utils::bind;
use std::fmt;
use std::future::poll_fn;
use std::io::{Error, IoSlice, Result, Write};
use std::net::{Shutdown, SocketAddr, ToSocketAddrs};
use std::pin::Pin;
use std::task::{Context, Poll};

use super::split::{TcpReader, TcpWriter, split};

pub struct TcpStream(pub(crate) AsyncIO<mio::net::TcpStream>);

impl TcpStream {
    pub(crate) fn new(io: mio::net::TcpStream) -> Result<TcpStream> {
        Ok(Self(AsyncIO::new(io)?))
    }

    pub async fn connect<A>(addr: A) -> Result<TcpStream>
    where
        A: ToSocketAddrs,
    {
        bind(addr, Self::connect_addr)?.connect_me().await
    }

    pub(crate) async fn connect_me(self) -> Result<TcpStream> {
        self.0.io_writable().await;

        if let Some(e) = self.0.io.take_error()? {
            return Err(e);
        }
        Ok(self)
    }

    /// Establishes a connection to the specified `addr`.
    fn connect_addr(addr: SocketAddr) -> Result<TcpStream> {
        TcpStream::new(mio::net::TcpStream::connect(addr)?)
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.0.io.local_addr()
    }

    /// Returns the value of the `SO_ERROR` option.
    pub fn take_error(&self) -> Result<Option<Error>> {
        self.0.io.take_error()
    }

    pub fn peer_addr(&self) -> Result<SocketAddr> {
        self.0.io.peer_addr()
    }

    pub fn peek<'b>(
        &mut self,
        buf: &'b mut [u8],
    ) -> impl Future<Output = Result<usize>> + use<'_, 'b> {
        self.0.io_read(|io| io.peek(buf))
    }

    pub fn nodelay(&self) -> Result<bool> {
        self.0.io.nodelay()
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.0.io.shutdown(how)
    }

    pub fn set_nodelay(&self, nodelay: bool) -> Result<()> {
        self.0.io.set_nodelay(nodelay)
    }

    pub fn ttl(&self) -> Result<u32> {
        self.0.io.ttl()
    }

    pub fn set_ttl(&self, ttl: u32) -> Result<()> {
        self.0.io.set_ttl(ttl)
    }

    pub fn split(self) -> (TcpReader, TcpWriter) {
        split(self)
    }

    pub fn read<'b>(
        &mut self,
        buf: &'b mut [u8],
    ) -> impl Future<Output = Result<usize>> + use<'_, 'b> {
        poll_fn(|cx| self.0.poll_read(cx, buf))
    }

    pub fn write<'b>(
        &mut self,
        buf: &'b [u8],
    ) -> impl Future<Output = Result<usize>> + use<'_, 'b> {
        poll_fn(|cx| self.0.poll_write(cx, buf))
    }

    pub fn write_vectored<'b>(
        &mut self,
        bufs: &'b [IoSlice],
    ) -> impl Future<Output = Result<usize>> + use<'_, 'b> {
        self.0
            .io_write(|mut io| Write::write_vectored(&mut io, bufs))
    }

    #[inline]
    pub(crate) fn poll_read(&self, cx: &mut Context, buf: &mut [u8]) -> Poll<Result<usize>> {
        self.0.poll_read(cx, buf)
    }

    #[inline]
    pub(crate) fn poll_write(&self, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>> {
        self.0.poll_write(cx, buf)
    }

    #[inline]
    pub(crate) fn poll_write_vectored(
        &self,
        cx: &mut Context,
        bufs: &[IoSlice],
    ) -> Poll<Result<usize>> {
        let mut poll_fn = self
            .0
            .io_write(|mut io| Write::write_vectored(&mut io, bufs));

        Pin::new(&mut poll_fn).poll(cx)
    }
}

impl TryFrom<std::net::TcpStream> for TcpStream {
    type Error = Error;
    fn try_from(stream: std::net::TcpStream) -> Result<Self> {
        stream.set_nonblocking(true)?;
        TcpStream::new(mio::net::TcpStream::from_std(stream))
    }
}

impl fmt::Debug for TcpStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
