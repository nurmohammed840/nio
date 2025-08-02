use std::{
    error::Error,
    fmt,
    future::Future,
    io::{IoSlice, IoSliceMut, Result},
    net::{Shutdown, SocketAddr},
    pin::Pin,
    sync::Arc,
    task::{ready, Context, Poll},
};

use crate::net::TcpStream;

#[derive(Debug)]
pub struct ReadHalf<'a> {
    me: &'a TcpStream,
}

#[derive(Debug)]
pub struct WriteHalf<'a> {
    me: &'a TcpStream,
}

pub fn split(stream: &mut TcpStream) -> (ReadHalf<'_>, WriteHalf<'_>) {
    (ReadHalf { me: stream }, WriteHalf { me: stream })
}

#[derive(Debug)]
pub struct OwnedReadHalf {
    me: Arc<TcpStream>,
}

#[derive(Debug)]
pub struct OwnedWriteHalf {
    me: Arc<TcpStream>,
    shutdown_on_drop: bool,
}

#[derive(Debug)]
pub struct ReuniteError(pub OwnedReadHalf, pub OwnedWriteHalf);

impl Error for ReuniteError {}
impl fmt::Display for ReuniteError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("tried to reunite halves that are not from the same socket")
    }
}

pub(crate) fn split_owned(stream: TcpStream) -> (OwnedReadHalf, OwnedWriteHalf) {
    #[allow(clippy::arc_with_non_send_sync)]
    let me = Arc::new(stream);
    let read = OwnedReadHalf {
        me: Arc::clone(&me),
    };
    let write = OwnedWriteHalf {
        me,
        shutdown_on_drop: true,
    };
    (read, write)
}

pub(crate) fn reunite(
    read: OwnedReadHalf,
    write: OwnedWriteHalf,
) -> std::result::Result<TcpStream, ReuniteError> {
    if Arc::ptr_eq(&read.me, &write.me) {
        write.forget();
        // This unwrap cannot fail as the api does not allow creating more than two Arcs,
        // and we just dropped the other half.
        Ok(Arc::try_unwrap(read.me).expect("TcpStream: try_unwrap failed in reunite"))
    } else {
        Err(ReuniteError(read, write))
    }
}

macro_rules! impl_split {
    ($reader: ty; $writer: ty; $shutdown: item) => {
        impl $reader {
            pub fn peer_addr(&self) -> Result<SocketAddr> {
                self.me.peer_addr()
            }

            pub fn local_addr(&self) -> Result<SocketAddr> {
                self.me.local_addr()
            }

            pub fn peek<'b>(&mut self, buf: &'b mut [u8])  -> impl Future<Output = Result<usize>> + use<'_, 'b> {
                self.me.peek(buf)
            }

            pub fn readable(&self) -> impl Future<Output = Result<()>> + '_ {
                self.me.readable()
            }

            pub fn try_read(&self, buf: &mut [u8]) -> Result<usize> {
                self.me.try_read(buf)
            }

            pub fn try_read_vectored(&self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
                self.me.try_read_vectored(bufs)
            }
        }

        impl $writer {
            pub fn peer_addr(&self) -> Result<SocketAddr> {
                self.me.peer_addr()
            }

            pub fn local_addr(&self) -> Result<SocketAddr> {
                self.me.local_addr()
            }

            pub fn writable(&self) -> impl Future<Output = Result<()>> + '_ {
                self.me.writable()
            }

            pub fn try_write(&self, buf: &[u8]) -> Result<usize> {
                self.me.try_write(buf)
            }

            pub fn try_write_vectored(&self, bufs: &[IoSlice<'_>]) -> Result<usize> {
                self.me.try_write_vectored(bufs)
            }
        }

        impl tokio::io::AsyncRead for $reader {
            fn poll_read(self: Pin<&mut Self>,cx: &mut Context, buf: &mut tokio::io::ReadBuf) -> Poll<Result<()>> {
                unsafe {
                    let b = &mut *(buf.unfilled_mut() as *mut _ as *mut [u8]);
                    let n = ready!(self.me.poll_read_priv(cx, b))?;
                    buf.assume_init(n);
                    buf.advance(n);
                    Poll::Ready(Ok(()))
                }
            }
        }

        impl tokio::io::AsyncWrite for $writer {
            fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>> {
                self.me.poll_write_priv(cx, buf)
            }

            fn poll_write_vectored(self: Pin<&mut Self>, cx: &mut Context, bufs: &[IoSlice]) -> Poll<Result<usize>> {
                self.me.poll_write_vectored_priv(cx, bufs)
            }

            fn is_write_vectored(&self) -> bool {
                self.me.is_write_vectored()
            }

            #[inline]
            fn poll_flush(self: Pin<&mut Self>, _: &mut Context) -> Poll<Result<()>> {
                // tcp flush is a no-op
                Poll::Ready(Ok(()))
            }

            // `poll_shutdown` on a write half shutdowns the stream in the "write" direction.
            $shutdown
        }

        impl AsRef<TcpStream> for $reader {
            fn as_ref(&self) -> &TcpStream {
                &self.me
            }
        }

        impl AsRef<TcpStream> for $writer {
            fn as_ref(&self) -> &TcpStream {
                &self.me
            }
        }

        unsafe impl Send for $reader {}
        unsafe impl Send for $writer {}
    };
}

impl_split! {
    ReadHalf<'_>; WriteHalf<'_>;
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context) -> Poll<Result<()>> {
        self.me.io.shutdown(Shutdown::Write).into()
    }
}

impl_split! {
    OwnedReadHalf; OwnedWriteHalf;
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context) -> Poll<Result<()>> {
        let res = self.me.io.shutdown(Shutdown::Write);
        if res.is_ok() {
            Pin::into_inner(self).shutdown_on_drop = false;
        }
        res.into()
    }
}

impl OwnedReadHalf {
    /// Attempts to put the two halves of a `TcpStream` back together and
    /// recover the original socket. Succeeds only if the two halves
    /// originated from the same call to [`into_split`].
    ///
    /// [`into_split`]: TcpStream::into_split()
    pub fn reunite(self, other: OwnedWriteHalf) -> std::result::Result<TcpStream, ReuniteError> {
        reunite(self, other)
    }
}

impl OwnedWriteHalf {
    pub fn reunite(self, other: OwnedReadHalf) -> std::result::Result<TcpStream, ReuniteError> {
        reunite(other, self)
    }

    pub fn forget(mut self) {
        self.shutdown_on_drop = false;
        drop(self);
    }
}

impl Drop for OwnedWriteHalf {
    fn drop(&mut self) {
        if self.shutdown_on_drop {
            let _ = self.me.io.shutdown(Shutdown::Write);
        }
    }
}
