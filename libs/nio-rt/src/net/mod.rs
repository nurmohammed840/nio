mod tcp;
mod udp;
mod utils;

pub use tcp::{
    listener::{TcpConnection, TcpListener},
    split::{TcpReader, TcpWriter},
    stream::TcpStream,
};
pub use udp::UdpSocket;

#[cfg(any(feature = "futures-io", feature = "tokio-io"))]
use std::{
    io::{IoSlice, Result},
    pin::Pin,
    task::{Context, Poll},
};

macro_rules! impl_async_read {
    [$($name:ty),*] => [$(
        #[cfg(feature = "futures-io")]
        impl futures_io::AsyncRead for $name {
            #[inline]
            fn poll_read(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                buf: &mut [u8],
            ) -> Poll<Result<usize>> {
                Self::poll_read(&self, cx, buf)
            }
        }

        #[cfg(feature = "tokio-io")]
        impl tokio::io::AsyncRead for $name {
            fn poll_read(
                self: Pin<&mut Self>,
                cx: &mut Context,
                buf: &mut tokio::io::ReadBuf,
            ) -> Poll<Result<()>> {
                unsafe {
                    let b = &mut *(buf.unfilled_mut() as *mut _ as *mut [u8]);
                    let n = std::task::ready!(Self::poll_read(&self, cx, b))?;
                    buf.assume_init(n);
                    buf.advance(n);
                    Poll::Ready(Ok(()))
                }
            }
        }

    )*]
}

macro_rules! impl_async_write {
    [$($name:ty),*] => [$(
        #[cfg(feature = "futures-io")]
        impl futures_io::AsyncWrite for $name {
            #[inline]
            fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
                Self::poll_write(&self, cx, buf)
            }

            #[inline]
            fn poll_write_vectored(
                self: Pin<&mut Self>,
                cx: &mut Context,
                bufs: &[IoSlice],
            ) -> Poll<Result<usize>> {
                Self::poll_write_vectored(&self, cx, bufs)
            }

            #[inline]
            fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<()>> {
                Poll::Ready(Ok(()))
            }

            #[inline]
            fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<()>> {
                self.shutdown(std::net::Shutdown::Write)?;
                Poll::Ready(Ok(()))
            }
        }

        #[cfg(feature = "tokio-io")]
        impl tokio::io::AsyncWrite for $name {
            #[inline]
            fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>> {
                Self::poll_write(&self, cx, buf)
            }

            #[inline]
            fn poll_write_vectored(
                self: Pin<&mut Self>,
                cx: &mut Context,
                bufs: &[IoSlice],
            ) -> Poll<Result<usize>> {
                Self::poll_write_vectored(&self, cx, bufs)
            }

            #[inline]
            fn is_write_vectored(&self) -> bool {
                true
            }

            #[inline]
            fn poll_flush(self: Pin<&mut Self>, _: &mut Context) -> Poll<Result<()>> {
                Poll::Ready(Ok(()))
            }

            #[inline]
            fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context) -> Poll<Result<()>> {
                self.shutdown(std::net::Shutdown::Write)?;
                Poll::Ready(Ok(()))
            }
        }

    )*]
}

impl_async_read! {
    TcpStream, TcpReader
}

impl_async_write! {
    TcpStream, TcpWriter
}
