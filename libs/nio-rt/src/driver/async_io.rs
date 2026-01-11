use crate::{driver::IoWaker, rt::context::LocalContext};
use mio::{Interest, event::Source};
use std::{
    future::{PollFn, poll_fn},
    io::{ErrorKind, Read, Result, Write},
    task::{Context, Poll},
};

#[derive(Debug)]
pub struct AsyncIO<Io: Source> {
    pub(crate) io: Io,
    waker: Box<IoWaker>,
}

impl<Io: Source> AsyncIO<Io> {
    pub fn new(io: Io) -> Result<Self> {
        Self::with_interest(io, Interest::READABLE | Interest::WRITABLE)
    }

    pub fn with_interest(mut io: Io, interests: Interest) -> Result<Self> {
        let waker = IoWaker::new();
        let token = mio::Token(waker.addr());

        LocalContext::with(|ctx| ctx.io_registry.register(&mut io, token, interests))?;

        Ok(Self { io, waker })
    }

    pub fn io_read<'a, F, T>(
        &'a self,
        mut f: F,
    ) -> PollFn<impl FnMut(&mut Context) -> Poll<Result<T>> + use<'a, F, Io, T>>
    where
        F: FnMut(&'a Io) -> Result<T>,
    {
        poll_fn(move |cx| {
            self.waker.reader.register(cx);

            let readiness = self.waker.readiness();
            if readiness.is_readable() {
                match f(&self.io) {
                    Err(err) if err.kind() == ErrorKind::WouldBlock => {
                        self.waker.clear_read(readiness)
                    }
                    res => return Poll::Ready(res),
                }
            }
            Poll::Pending
        })
    }

    pub fn io_write<'a, F, T>(
        &'a self,
        mut f: F,
    ) -> PollFn<impl FnMut(&mut Context) -> Poll<Result<T>> + use<'a, F, Io, T>>
    where
        F: FnMut(&'a Io) -> Result<T>,
    {
        poll_fn(move |cx| {
            self.waker.writer.register(cx);

            let readiness = self.waker.readiness();
            if readiness.is_writable() {
                match f(&self.io) {
                    Err(err) if err.kind() == ErrorKind::WouldBlock => {
                        self.waker.clear_write(readiness)
                    }
                    res => return Poll::Ready(res),
                }
            }
            Poll::Pending
        })
    }

    pub fn io_writable(&self) -> PollFn<impl FnMut(&mut Context) -> Poll<()> + use<'_, Io>> {
        poll_fn(move |cx| {
            self.waker.writer.register(cx);
            if self.waker.readiness().is_writable() {
                return Poll::Ready(());
            }
            Poll::Pending
        })
    }
}

impl<Io: Source> AsyncIO<Io> {
    pub fn poll_read<'a>(&'a self, cx: &mut Context, buf: &mut [u8]) -> Poll<Result<usize>>
    where
        &'a Io: Read,
    {
        self.waker.reader.register(cx);

        let readiness = self.waker.readiness();
        if readiness.is_readable() {
            match Read::read(&mut &self.io, buf) {
                Ok(nbytes) => {
                    // When mio is using the epoll or kqueue selector, reading a partially full
                    // buffer is sufficient to show that the socket buffer has been drained.
                    //
                    // This optimization does not work for level-triggered selectors such as
                    // windows or when poll is used.
                    //
                    // Read more:
                    // https://github.com/tokio-rs/tokio/issues/5866
                    #[cfg(all(not(mio_unsupported_force_poll_poll), not(windows)))]
                    if 0 < nbytes && nbytes < buf.len() {
                        // same as: nbytes in 1..buf.len()
                        self.waker.clear_read(readiness);
                    }
                    return Poll::Ready(Ok(nbytes));
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => self.waker.clear_read(readiness),
                Err(err) => return Poll::Ready(Err(err)),
            }
        }
        Poll::Pending
    }

    pub fn poll_write<'a>(&'a self, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>>
    where
        &'a Io: Write,
    {
        self.waker.writer.register(cx);

        let readiness = self.waker.readiness();
        if readiness.is_writable() {
            match Write::write(&mut &self.io, buf) {
                Ok(nbytes) => {
                    // if we write only part of our buffer, this is sufficient on unix to show
                    // that the socket buffer is full.  Unfortunately this assumption
                    // fails for level-triggered selectors (like on Windows or poll even for
                    // UNIX): https://github.com/tokio-rs/tokio/issues/5866
                    #[cfg(all(not(mio_unsupported_force_poll_poll), not(windows)))]
                    if 0 < nbytes && nbytes < buf.len() {
                        self.waker.clear_write(readiness);
                    }
                    return Poll::Ready(Ok(nbytes));
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => {
                    self.waker.clear_write(readiness)
                }
                Err(err) => return Poll::Ready(Err(err)),
            }
        }
        Poll::Pending
    }
}

impl<Io: Source> Drop for AsyncIO<Io> {
    fn drop(&mut self) {
        LocalContext::with(|ctx| {
            let _ = ctx.io_registry.deregister(&mut self.io);
        });
        self.waker.drop_waker();
    }
}
