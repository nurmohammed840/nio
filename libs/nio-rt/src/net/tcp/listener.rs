use crate::{
    driver::AsyncIO,
    net::{TcpStream, utils::bind},
};
use std::{
    fmt, future,
    io::{Error, Result},
    net::{SocketAddr, ToSocketAddrs},
};

pub struct TcpListener(AsyncIO<mio::net::TcpListener>);

impl TcpListener {
    pub fn new(io: mio::net::TcpListener) -> Result<TcpListener> {
        Ok(TcpListener(AsyncIO::new(io)?))
    }

    fn bind_addr(addr: SocketAddr) -> Result<TcpListener> {
        TcpListener::new(mio::net::TcpListener::bind(addr)?)
    }

    pub fn bind<A: ToSocketAddrs>(addr: A) -> impl Future<Output = Result<TcpListener>> {
        future::ready(bind(addr, TcpListener::bind_addr))
    }

    pub fn accept(&mut self) -> impl Future<Output = Result<TcpConnection>> + '_ {
        self.0.io_read(|io| {
            let (stream, addr) = io.accept()?;
            Ok(TcpConnection::new(addr, stream))
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.0.io.local_addr()
    }

    pub fn ttl(&self) -> Result<u32> {
        self.0.io.ttl()
    }

    pub fn set_ttl(&self, ttl: u32) -> Result<()> {
        self.0.io.set_ttl(ttl)
    }

    pub fn take_error(&self) -> Result<Option<Error>> {
        self.0.io.take_error()
    }
}

pub struct TcpConnection {
    addr: SocketAddr,
    stream: mio::net::TcpStream,
}

impl TcpConnection {
    pub(crate) fn new(addr: SocketAddr, stream: mio::net::TcpStream) -> Self {
        Self { addr, stream }
    }

    pub fn peer_addr(&self) -> Result<SocketAddr> {
        Ok(self.addr)
    }

    pub fn nodelay(&self) -> Result<bool> {
        self.stream.nodelay()
    }

    pub fn set_nodelay(&self, nodelay: bool) -> Result<()> {
        self.stream.set_nodelay(nodelay)
    }

    pub fn ttl(&self) -> Result<u32> {
        self.stream.ttl()
    }

    pub fn set_ttl(&self, ttl: u32) -> Result<()> {
        self.stream.set_ttl(ttl)
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.stream.local_addr()
    }

    pub fn connect(self) -> impl Future<Output = Result<TcpStream>> {
        future::ready(TcpStream::new(self.stream))
    }
}

impl fmt::Debug for TcpListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.io.fmt(f)
    }
}
