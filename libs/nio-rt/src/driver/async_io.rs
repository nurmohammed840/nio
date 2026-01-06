use crate::driver::io_waker::Readiness;
use crate::{driver::IoWaker, rt::context::LocalContext};
use mio::Interest;
use mio::event::Source;
use std::future::{PollFn, poll_fn};
use std::io::{ErrorKind, Result};
use std::rc::Rc;
use std::task::{Context, Poll};

#[derive(Debug)]
struct AsyncIO<Io: Source> {
    io: Io,
    waker: Box<IoWaker>,
}

impl<Io: Source> AsyncIO<Io> {
    pub fn new(io: Io) -> Result<Self> {
        Self::with_interest(io, Interest::READABLE | Interest::WRITABLE)
    }

    pub fn with_interest(mut io: Io, interests: Interest) -> Result<Self> {
        let waker = IoWaker::new();
        let token = mio::Token(waker.addr());

        LocalContext::with(|ctx| {
            ctx.io_registry.register(&mut io, token, interests);
        });

        Ok(Self { io, waker })
    }

    pub fn async_io_read<F, T>(
        &self,
        mut f: F,
    ) -> PollFn<impl FnMut(&mut Context) -> Poll<Result<T>> + use<'_, F, Io, T>>
    where
        F: FnMut(&Io) -> Result<T>,
    {
        poll_fn(move |cx| {
            self.waker.reader.register(cx);
            
            let readiness = self.waker.readiness();
            if readiness.is_readable() {
                match f(&self.io) {
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                        self.waker.clear_read(readiness)
                    }
                    res => return Poll::Ready(res),
                }
            }
            Poll::Pending
        })
    }

    pub fn async_io_write<F, T>(
        &self,
        mut f: F,
    ) -> PollFn<impl FnMut(&mut Context) -> Poll<Result<T>> + use<'_, F, Io, T>>
    where
        F: FnMut(&Io) -> Result<T>,
    {
        poll_fn(move |cx| {
            self.waker.writer.register(cx);

            let readiness = self.waker.readiness();
            if readiness.is_writable() {
                match f(&self.io) {
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                        self.waker.clear_write(readiness)
                    }
                    res => return Poll::Ready(res),
                }
            }
            Poll::Pending
        })
    }

    pub fn poll_read_readiness(
        &self,
    ) -> PollFn<impl FnMut(&mut Context) -> Poll<()> + use<'_, Io>> {
        poll_fn(move |cx| {
            self.waker.reader.register(cx);
            if self.waker.readiness().is_readable() {
                return Poll::Ready(());
            }
            Poll::Pending
        })
    }

    pub fn poll_write_readiness(
        &self,
    ) -> PollFn<impl FnMut(&mut Context) -> Poll<()> + use<'_, Io>> {
        poll_fn(move |cx| {
            self.waker.writer.register(cx);
            if self.waker.readiness().is_writable() {
                return Poll::Ready(());
            }
            Poll::Pending
        })
    }
}

impl<Io: Source> Drop for AsyncIO<Io> {
    fn drop(&mut self) {
        LocalContext::with(|ctx| {
            ctx.io_registry.deregister(&mut self.io);
        });
        self.waker.drop_waker();
    }
}
