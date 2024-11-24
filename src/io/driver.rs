#![allow(dead_code)]
use std::{io, time::Duration};

pub struct Events {
    poll: mio::Poll,
    events: mio::Events,
}

pub const WAKE_TOKEN: mio::Token = mio::Token(0);

impl Events {
    pub fn with_capacity(capacity: usize) -> io::Result<(Self, mio::Waker)> {
        let poll = mio::Poll::new()?;
        let waker = mio::Waker::new(poll.registry(), WAKE_TOKEN)?;
        Ok((
            Self {
                poll,
                events: mio::Events::with_capacity(capacity),
            },
            waker,
        ))
    }

    pub fn is_wake(ev: &mio::event::Event) -> bool {
        ev.token() == WAKE_TOKEN
    }

    pub fn registry(&self) -> &mio::Registry {
        self.poll.registry()
    }

    pub fn registry_owned(&self) -> io::Result<mio::Registry> {
        self.poll.registry().try_clone()
    }

    pub fn is_empty(&mut self) -> bool {
        self.events.is_empty()
    }

    pub fn poll(&mut self, timeout: Option<Duration>) -> io::Result<&mio::Events> {
        self.poll.poll(&mut self.events, timeout)?;
        Ok(&self.events)
    }
}
