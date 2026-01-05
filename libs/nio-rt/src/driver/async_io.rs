use crate::{driver::io::IoWaker, rt::context::LocalContext};
use mio::Interest;
use mio::event::Source;
use std::io::Result;
use std::rc::Rc;

struct AsyncIO<Io> {
    io: Io,
    waker: Rc<IoWaker>,
}

impl<Io: Source> AsyncIO<Io> {
    pub fn new(io: Io) -> Result<Self> {
        Self::with_interest(io, Interest::READABLE | Interest::WRITABLE)
    }

    fn with_interest(mut io: Io, interests: Interest) -> Result<Self> {
        let waker = IoWaker::new();
        let token = mio::Token(waker.addr());

        LocalContext::with(|ctx| {
            ctx.io_registry.register(&mut io, token, interests);
        });

        Ok(Self { io, waker })
    }
}
