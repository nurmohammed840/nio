use crate::net::TcpStream;
use std::io::{Error, IoSlice};
use std::net::{Shutdown, SocketAddr};
use std::rc::Rc;
use std::task::{Context, Poll};

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct TcpReader(Rc<TcpStream>);

#[derive(Debug)]
pub struct TcpWriter {
    stream: Rc<TcpStream>,
    shutdown_on_drop: bool,
}

pub(crate) fn split(stream: TcpStream) -> (TcpReader, TcpWriter) {
    let stream = Rc::new(stream);
    (
        TcpReader(stream.clone()),
        TcpWriter {
            stream,
            shutdown_on_drop: true,
        },
    )
}

pub(crate) fn reunite(
    read: TcpReader,
    write: TcpWriter,
) -> Result<TcpStream, (TcpReader, TcpWriter)> {
    if Rc::ptr_eq(&read.0, &write.stream) {
        write.drop_without_shutdown();
        // This unwrap cannot fail as the api does not allow creating more than two Rcs,
        // and we just dropped the other half.
        Ok(Rc::try_unwrap(read.0).expect("TcpStream: try_unwrap failed in reunite"))
    } else {
        Err((read, write))
    }
}

impl TcpReader {
    #[inline]
    pub fn peer_addr(&self) -> Result<SocketAddr> {
        self.0.peer_addr()
    }

    #[inline]
    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.0.local_addr()
    }

    #[inline]
    pub fn peek<'b>(
        &mut self,
        buf: &'b mut [u8],
    ) -> impl Future<Output = Result<usize>> + use<'_, 'b> {
        self.0.peek(buf)
    }

    #[inline]
    pub fn poll_read(&self, cx: &mut Context, buf: &mut [u8]) -> Poll<Result<usize>> {
        self.0.poll_read(cx, buf)
    }
}

impl TcpWriter {
    #[inline]
    pub(crate) fn shutdown(&self, how: std::net::Shutdown) -> Result<()> {
        self.stream.shutdown(how)
    }

    #[inline]
    pub fn peer_addr(&self) -> Result<SocketAddr> {
        self.stream.peer_addr()
    }

    #[inline]
    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.stream.local_addr()
    }

    #[inline]
    pub fn poll_write(&self, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>> {
        self.stream.poll_write(cx, buf)
    }

    #[inline]
    pub fn poll_write_vectored(&self, cx: &mut Context, bufs: &[IoSlice]) -> Poll<Result<usize>> {
        self.stream.poll_write_vectored(cx, bufs)
    }
}

impl TcpReader {
    pub fn reunite(self, other: TcpWriter) -> Result<TcpStream, (TcpReader, TcpWriter)> {
        reunite(self, other)
    }
}

impl TcpWriter {
    pub fn reunite(self, other: TcpReader) -> Result<TcpStream, (TcpReader, TcpWriter)> {
        reunite(other, self)
    }

    pub fn drop_without_shutdown(mut self) {
        self.shutdown_on_drop = false;
        drop(self);
    }
}

impl Drop for TcpWriter {
    fn drop(&mut self) {
        if self.shutdown_on_drop {
            let _ = self.stream.shutdown(Shutdown::Write);
        }
    }
}
