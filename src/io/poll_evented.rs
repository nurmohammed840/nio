use crate::runtime::Handle;

use super::scheduled_io::{Readiness, ScheduledIo};
use mio::{event::Source, Interest};
use std::{
    cell::Cell,
    fmt::{self},
    future::{poll_fn, PollFn},
    io::{self, ErrorKind, IoSlice, Result},
    marker::PhantomData,
    ops::Deref,
    sync::Arc,
    task::{Context, Poll},
};

pub struct PollEvented<E: Source> {
    io: Option<E>,
    registration: Registration,
    // Not `!Sync`
    _p: PhantomData<Cell<()>>,
}

pub struct Registration {
    handle: Handle,
    scheduled_io: Arc<ScheduledIo>,
}

impl<E: Source> PollEvented<E> {
    pub fn new(io: E) -> Result<Self> {
        Self::with_interest(io, Interest::READABLE | Interest::WRITABLE)
    }

    pub fn with_interest(mut io: E, interest: Interest) -> Result<Self> {
        let handle = Handle::current();
        let scheduled_io = handle.ctx.reactor().register(&mut io, interest)?;
        Ok(Self {
            io: Some(io),
            registration: Registration {
                handle,
                scheduled_io,
            },
            _p: PhantomData,
        })
    }

    fn _into_inner(mut self) -> Result<E> {
        let mut io = self.io.take().unwrap();
        self.drop_registration(&mut io)?;
        Ok(io)
    }

    fn drop_registration(&mut self, io: &mut E) -> Result<()> {
        let reactor_ctx = self.registration.handle.ctx.reactor();
        reactor_ctx.deregister(io, &self.registration.scheduled_io)?;
        self.registration.scheduled_io.drop_wakers();
        Ok(())
    }

    pub fn async_io_read<F, T>(
        &self,
        mut f: F,
    ) -> PollFn<impl FnMut(&mut Context) -> Poll<io::Result<T>> + use<'_, E, T, F>>
    where
        F: FnMut(&E) -> Result<T>,
    {
        let io = self.deref();
        let scheduled_io = &self.registration.scheduled_io;

        poll_fn(move |cx| {
            scheduled_io.read_event.waker.register(cx.waker());

            let mut readiness = scheduled_io.read_readiness();
            if readiness.is_empty() {
                return Poll::Pending;
            }

            loop {
                match f(io) {
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                        match scheduled_io.clear_read_readiness(readiness) {
                            Some(new) => readiness = new,
                            None => return Poll::Pending,
                        }
                    }
                    res => return Poll::Ready(res),
                }
            }
        })
    }

    pub fn async_io_write<F, T>(
        &self,
        mut f: F,
    ) -> PollFn<impl FnMut(&mut Context) -> Poll<io::Result<T>> + use<'_, E, T, F>>
    where
        F: FnMut(&E) -> Result<T>,
    {
        let io = self.deref();
        let scheduled_io = &self.registration.scheduled_io;

        poll_fn(move |cx| {
            scheduled_io.write_event.waker.register(cx.waker());

            let mut readiness = scheduled_io.write_readiness();
            if readiness.is_empty() {
                return Poll::Pending;
            }

            loop {
                match f(io) {
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                        match scheduled_io.clear_write_readiness(readiness) {
                            Some(new) => readiness = new,
                            None => return Poll::Pending,
                        }
                    }
                    res => return Poll::Ready(res),
                }
            }
        })
    }

    pub fn try_io_read<R, F>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&E) -> Result<R>,
    {
        let readiness = self.registration.scheduled_io.read_readiness();

        if readiness.is_empty() {
            return Err(ErrorKind::WouldBlock.into());
        }

        match f(self.deref()) {
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                let _ = self
                    .registration
                    .scheduled_io
                    .clear_read_readiness(readiness);

                Err(e)
            }
            res => res,
        }
    }

    pub fn try_io_write<R, F>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&E) -> Result<R>,
    {
        let readiness = self.registration.scheduled_io.write_readiness();

        if readiness.is_empty() {
            return Err(ErrorKind::WouldBlock.into());
        }

        match f(self.deref()) {
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                let _ = self
                    .registration
                    .scheduled_io
                    .clear_write_readiness(readiness);

                Err(e)
            }
            res => res,
        }
    }

    pub fn poll_read_readiness(
        &self,
    ) -> PollFn<impl FnMut(&mut Context) -> Poll<Readiness> + use<'_, E>> {
        let scheduled_io = &self.registration.scheduled_io;
        poll_fn(move |cx| {
            scheduled_io.read_event.waker.register(cx.waker());

            let readiness = scheduled_io.read_readiness();
            if readiness.is_empty() {
                return Poll::Pending;
            }

            Poll::Ready(readiness)
        })
    }

    pub fn poll_write_readiness(
        &self,
    ) -> PollFn<impl FnMut(&mut Context) -> Poll<Readiness> + use<'_, E>> {
        let scheduled_io = &self.registration.scheduled_io;
        poll_fn(move |cx| {
            scheduled_io.write_event.waker.register(cx.waker());

            let readiness = scheduled_io.write_readiness();
            if readiness.is_empty() {
                return Poll::Pending;
            }

            Poll::Ready(readiness)
        })
    }
}

impl<E: Source> PollEvented<E> {
    // Safety: The caller must ensure that `E` can read into uninitialized memory
    pub unsafe fn poll_read<'a>(&'a self, cx: &mut Context, buf: &mut [u8]) -> Poll<Result<usize>>
    where
        &'a E: io::Read + 'a,
    {
        use std::io::Read;

        let scheduled_io = &self.registration.scheduled_io;
        scheduled_io.read_event.waker.register(cx.waker());

        let mut readiness = scheduled_io.read_readiness();
        if readiness.is_empty() {
            return Poll::Pending;
        }

        loop {
            // used only when the cfgs below apply
            #[allow(unused_variables)]
            let len = buf.len();

            match self.deref().read(buf) {
                Ok(n) => {
                    // When mio is using the epoll or kqueue selector, reading a partially full
                    // buffer is sufficient to show that the socket buffer has been drained.
                    //
                    // This optimization does not work for level-triggered selectors such as
                    // windows or when poll is used.
                    //
                    // Read more:
                    // https://github.com/tokio-rs/tokio/issues/5866
                    #[cfg(all(
                        not(mio_unsupported_force_poll_poll),
                        any(
                            // epoll
                            target_os = "android",
                            target_os = "illumos",
                            target_os = "linux",
                            target_os = "redox",
                            // kqueue
                            target_os = "dragonfly",
                            target_os = "freebsd",
                            target_os = "ios",
                            target_os = "macos",
                            target_os = "netbsd",
                            target_os = "openbsd",
                            target_os = "tvos",
                            target_os = "visionos",
                            target_os = "watchos",
                        )
                    ))]
                    if 0 < n && n < len {
                        let _ = scheduled_io.clear_read_readiness(readiness);
                    }
                    return Poll::Ready(Ok(n));
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    match scheduled_io.clear_read_readiness(readiness) {
                        Some(new) => readiness = new,
                        None => return Poll::Pending,
                    }
                }
                Err(e) => return Poll::Ready(Err(e)),
            }
        }
    }

    pub fn poll_write<'a>(&'a self, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>>
    where
        &'a E: io::Write + 'a,
    {
        use std::io::Write;

        let scheduled_io = &self.registration.scheduled_io;
        scheduled_io.write_event.waker.register(cx.waker());

        let mut readiness = scheduled_io.write_readiness();
        if readiness.is_empty() {
            return Poll::Pending;
        }

        loop {
            match self.deref().write(buf) {
                Ok(n) => {
                    // if we write only part of our buffer, this is sufficient on unix to show
                    // that the socket buffer is full.  Unfortunately this assumption
                    // fails for level-triggered selectors (like on Windows or poll even for
                    // UNIX): https://github.com/tokio-rs/tokio/issues/5866
                    if n > 0
                        && n < buf.len()
                        && (!cfg!(windows) && !cfg!(mio_unsupported_force_poll_poll))
                    {
                        let _ = scheduled_io.clear_write_readiness(readiness);
                    }
                    return Poll::Ready(Ok(n));
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    match scheduled_io.clear_write_readiness(readiness) {
                        Some(new) => readiness = new,
                        None => return Poll::Pending,
                    }
                }
                Err(e) => return Poll::Ready(Err(e)),
            }
        }
    }

    pub fn poll_write_vectored<'a>(
        &'a self,
        cx: &mut Context,
        bufs: &[IoSlice],
    ) -> Poll<Result<usize>>
    where
        &'a E: io::Write + 'a,
    {
        use std::io::Write;
        let scheduled_io = &self.registration.scheduled_io;
        scheduled_io.write_event.waker.register(cx.waker());

        let mut readiness = scheduled_io.write_readiness();
        if readiness.is_empty() {
            return Poll::Pending;
        }

        loop {
            match self.deref().write_vectored(bufs) {
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    match scheduled_io.clear_write_readiness(readiness) {
                        Some(new) => readiness = new,
                        None => return Poll::Pending,
                    }
                }
                res => return Poll::Ready(res),
            }
        }
    }
}

impl<E: Source + fmt::Debug> fmt::Debug for PollEvented<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.io.as_ref().unwrap().fmt(f)
    }
}

impl<E: Source> Deref for PollEvented<E> {
    type Target = E;
    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.io.as_ref().unwrap_unchecked() }
    }
}

impl<E: Source> Drop for PollEvented<E> {
    fn drop(&mut self) {
        if let Some(mut io) = self.io.take() {
            // Ignore errors
            let _ = self.drop_registration(&mut io);
        }
    }
}
