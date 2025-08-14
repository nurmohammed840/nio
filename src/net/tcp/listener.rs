use crate::{
    io::poll_evented::PollEvented,
    net::{utils::bind, TcpStream},
};
use std::{
    fmt,
    future::Future,
    io::Result,
    net::{SocketAddr, ToSocketAddrs},
};

pub struct TcpListener {
    io: PollEvented<mio::net::TcpListener>,
}

impl TcpListener {
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> Result<TcpListener> {
        bind(addr, TcpListener::bind_addr)
    }

    fn bind_addr(addr: SocketAddr) -> Result<TcpListener> {
        TcpListener::new(mio::net::TcpListener::bind(addr)?)
    }

    pub(crate) fn new(listener: mio::net::TcpListener) -> Result<TcpListener> {
        let io = PollEvented::new(listener)?;
        Ok(TcpListener { io })
    }

    pub fn accept(&self) -> impl Future<Output = Result<(TcpStream, SocketAddr)>> + '_ {
        self.io.async_io_read(|io| {
            let (mio, addr) = io.accept()?;
            Ok((TcpStream::new(mio)?, addr))
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.io.local_addr()
    }

    pub fn ttl(&self) -> Result<u32> {
        self.io.ttl()
    }

    pub fn set_ttl(&self, ttl: u32) -> Result<()> {
        self.io.set_ttl(ttl)
    }
}

impl fmt::Debug for TcpListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.io.fmt(f)
    }
}
