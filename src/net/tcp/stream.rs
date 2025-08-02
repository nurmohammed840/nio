use crate::{
    io::{poll_evented::PollEvented, Interest, Ready},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf, ReadHalf, WriteHalf},
        utils::bind,
    },
};
use std::{
    fmt,
    future::Future,
    io::{self, IoSlice, Result},
    net::{SocketAddr, ToSocketAddrs},
    pin::Pin,
    task::{ready, Context, Poll},
};

use super::split::{split, split_owned};

pub struct TcpStream {
    pub(crate) io: PollEvented<mio::net::TcpStream>,
}

impl TcpStream {
    pub async fn connect<A: ToSocketAddrs>(addr: A) -> Result<TcpStream> {
        bind(addr, Self::connect_addr)?.connect_me().await
    }

    pub(crate) async fn connect_me(self) -> Result<TcpStream> {
        // Once we've connected, wait for the stream to be writable as
        // that's when the actual connection has been initiated. Once we're
        // writable we check for `take_socket_error` to see if the connect
        // actually hit an error or not.
        //
        // If all that succeeded then we ship everything on up.
        self.io.readiness(Interest::WRITABLE, |_| {}).await;

        if let Some(e) = self.io.take_error()? {
            return Err(e);
        }
        Ok(self)
    }

    /// Establishes a connection to the specified `addr`.
    fn connect_addr(addr: SocketAddr) -> Result<TcpStream> {
        TcpStream::new(mio::net::TcpStream::connect(addr)?)
    }

    pub(crate) fn new(io: mio::net::TcpStream) -> Result<Self> {
        let io = PollEvented::new(io)?;
        Ok(Self { io })
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.io.local_addr()
    }

    /// Returns the value of the `SO_ERROR` option.
    pub fn take_error(&self) -> Result<Option<io::Error>> {
        self.io.take_error()
    }

    pub fn peer_addr(&self) -> Result<SocketAddr> {
        self.io.peer_addr()
    }

    pub fn peek<'b>(&self, buf: &'b mut [u8]) -> impl Future<Output = Result<usize>> + use<'_, 'b> {
        self.io.async_io(Interest::READABLE, |io| io.peek(buf))
    }

    pub fn ready(&self, interest: Interest) -> impl Future<Output = Result<Ready>> + '_ {
        self.io.readiness(interest, |e| Ok(e.ready))
    }

    pub fn readable(&self) -> impl Future<Output = Result<()>> + '_ {
        self.io.readiness(Interest::READABLE, |_| Ok(()))
    }

    pub fn try_read(&self, buf: &mut [u8]) -> Result<usize> {
        use std::io::Read;
        self.io.try_io(Interest::READABLE, |mut io| io.read(buf))
    }

    pub fn try_read_vectored(&self, bufs: &mut [io::IoSliceMut<'_>]) -> Result<usize> {
        use std::io::Read;
        self.io
            .try_io(Interest::READABLE, |mut io| io.read_vectored(bufs))
    }

    pub fn writable(&self) -> impl Future<Output = Result<()>> + '_ {
        self.io.readiness(Interest::WRITABLE, |_| Ok(()))
    }

    pub fn try_write(&self, buf: &[u8]) -> Result<usize> {
        use std::io::Write;
        self.io.try_io(Interest::WRITABLE, |mut io| io.write(buf))
    }

    pub fn try_write_vectored(&self, bufs: &[IoSlice]) -> Result<usize> {
        use std::io::Write;
        self.io
            .try_io(Interest::WRITABLE, |mut io| io.write_vectored(bufs))
    }

    pub fn try_io<F, T>(&self, interest: Interest, f: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>,
    {
        self.io.try_io(interest, |io| io.try_io(f))
    }

    pub fn async_io<F, T>(
        &self,
        interest: Interest,
        mut f: F,
    ) -> impl Future<Output = Result<T>> + use<'_, F, T>
    where
        F: FnMut() -> Result<T>,
    {
        self.io.async_io(interest, move |io| io.try_io(&mut f))
    }

    pub fn nodelay(&self) -> Result<bool> {
        self.io.nodelay()
    }

    pub fn set_nodelay(&self, nodelay: bool) -> Result<()> {
        self.io.set_nodelay(nodelay)
    }

    pub fn ttl(&self) -> Result<u32> {
        self.io.ttl()
    }

    pub fn set_ttl(&self, ttl: u32) -> Result<()> {
        self.io.set_ttl(ttl)
    }

    pub fn split(&mut self) -> (ReadHalf<'_>, WriteHalf<'_>) {
        split(self)
    }

    pub fn into_split(self) -> (OwnedReadHalf, OwnedWriteHalf) {
        split_owned(self)
    }

    // === Poll IO functions that takes `&self` ===
    //
    // To read or write without mutable access to the `TcpStream`, combine the
    // `poll_read_ready` or `poll_write_ready` methods with the `try_read` or
    // `try_write` methods.

    pub(crate) fn poll_read_priv(&self, cx: &mut Context, buf: &mut [u8]) -> Poll<Result<usize>> {
        // Safety: `TcpStream::read` correctly handles reads into uninitialized memory
        unsafe { self.io.poll_read(cx, buf) }
    }

    pub(super) fn poll_write_priv(&self, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>> {
        self.io.poll_write(cx, buf)
    }

    pub(super) fn poll_write_vectored_priv(
        &self,
        cx: &mut Context,
        bufs: &[IoSlice],
    ) -> Poll<Result<usize>> {
        self.io.poll_write_vectored(cx, bufs)
    }
}

impl tokio::io::AsyncRead for TcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut tokio::io::ReadBuf,
    ) -> Poll<Result<()>> {
        unsafe {
            let b = &mut *(buf.unfilled_mut() as *mut _ as *mut [u8]);
            let n = ready!(self.poll_read_priv(cx, b))?;
            buf.assume_init(n);
            buf.advance(n);
            Poll::Ready(Ok(()))
        }
    }
}

impl tokio::io::AsyncWrite for TcpStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>> {
        self.poll_write_priv(cx, buf)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context,
        bufs: &[IoSlice],
    ) -> Poll<Result<usize>> {
        self.poll_write_vectored_priv(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        true
    }

    #[inline]
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context) -> Poll<Result<()>> {
        // tcp flush is a no-op
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context) -> Poll<Result<()>> {
        self.io.shutdown(std::net::Shutdown::Write)?;
        Poll::Ready(Ok(()))
    }
}

impl fmt::Debug for TcpStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.io.fmt(f)
    }
}
