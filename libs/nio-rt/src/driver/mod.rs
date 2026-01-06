mod io_waker;
mod async_io;
use std::{io::Result, time::Duration};

pub use io_waker::IoWaker;
pub use async_io::AsyncIO;
pub use mio::{Events, Registry, Waker, event::Event};

pub struct Driver {
    poll: mio::Poll,
    events: Events,
}

pub const WAKE_TOKEN: mio::Token = mio::Token(0);

impl Driver {
    pub fn with_capacity(capacity: usize) -> Result<(Self, Waker)> {
        let poll = mio::Poll::new()?;
        let waker = Waker::new(poll.registry(), WAKE_TOKEN)?;
        Ok((
            Self {
                poll,
                events: Events::with_capacity(capacity),
            },
            waker,
        ))
    }

    pub fn has_woken(ev: &Event) -> bool {
        ev.token() == WAKE_TOKEN
    }
    
    pub fn registry(&self) -> &Registry {
        self.poll.registry()
    }

    pub fn registry_owned(&self) -> Result<Registry> {
        self.registry().try_clone()
    }

    pub fn _is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn poll(&mut self, timeout: Option<Duration>) -> Result<&Events> {
        self.poll.poll(&mut self.events, timeout)?;
        Ok(&self.events)
    }
}
