use std::io::{self, Result};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use mio::event::Source;

use super::driver::Events;
use super::scheduled_io::{ScheduledIo, Tick};
use super::{Interest, Ready};
use crate::time::timer::Timers;

pub struct Reactor {
    events: Events,
    shared: Arc<Shared>,
}

pub struct ReactorContext {
    waker: mio::Waker,
    registry: mio::Registry,
    pub shared: Arc<Shared>,
}

pub struct Shared {
    pub timers: Mutex<Timers>,
    pending: PendingRelease,
}

struct PendingRelease {
    release_len: AtomicUsize,
    release: Mutex<Vec<Arc<ScheduledIo>>>,
}

impl Reactor {
    pub fn new(capacity: usize) -> Result<(Reactor, ReactorContext)> {
        let (events, waker) = Events::with_capacity(capacity)?;
        let registry = events.registry_owned()?;

        let timers = Mutex::new(Timers::default());
        let pending = PendingRelease {
            release_len: AtomicUsize::new(0),
            release: Mutex::new(Vec::new()),
        };

        let shared = Arc::new(Shared { timers, pending });

        Ok((
            Reactor {
                events,
                shared: shared.clone(),
            },
            ReactorContext {
                waker,
                registry,
                shared,
            },
        ))
    }
}

impl Reactor {
    pub fn run(mut self) {
        loop {
            self.turn();
        }
    }

    pub fn turn(&mut self) {
        self.shared.pending.release_pending();

        let timeout = self.shared.timers.lock().unwrap().next_timeout();
        let events = match self.events.poll(timeout) {
            Ok(events) => events,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => return,
            Err(e) => panic!("unexpected error when polling the I/O driver: {e:?}"),
        };

        for event in events {
            if Events::is_wake(event) {
                continue;
            }

            let ready = Ready::from_mio(event);

            let token = event.token();
            let ptr = ScheduledIo::from_token(token.0);

            // Safety: we ensure that the pointers used as tokens are not freed
            // until they are both deregistered from mio **and** we know the I/O
            // driver is not concurrently polling. The I/O driver holds ownership of
            // an `Arc<ScheduledIo>` so we can safely cast this to a ref.
            let io: &ScheduledIo = unsafe { &*ptr };
            io.set_readiness(Tick::Set, |curr| curr | ready);
            io.wake(ready);
        }

        self.shared.timers.lock().unwrap().process();
    }
}

impl PendingRelease {
    fn is_empty(&self) -> bool {
        self.release_len.load(Ordering::Acquire) == 0
    }

    fn release_pending(&self) {
        if !self.is_empty() {
            let mut list = self.release.lock().unwrap();
            list.clear();
            self.release_len.store(0, Ordering::Release);
        }
    }
}

impl ReactorContext {
    pub fn wake(&self) {
        self.waker.wake().expect("failed to wake I/O driver");
    }

    pub fn register<S>(&self, io: &mut S, interest: Interest) -> Result<Arc<ScheduledIo>>
    where
        S: Source,
    {
        let scheduled_io = Arc::new(ScheduledIo::default());
        let token = mio::Token(scheduled_io.into_token());
        self.registry.register(io, token, interest.to_mio())?;
        Ok(scheduled_io)
    }

    pub fn deregister<E>(&self, io: &mut E, scheduled_io: &Arc<ScheduledIo>) -> Result<()>
    where
        E: Source,
    {
        self.registry.deregister(io)?;
        let len = {
            let mut pending_release = self.shared.pending.release.lock().unwrap();
            pending_release.push(scheduled_io.clone());

            let len = pending_release.len();
            self.shared
                .pending
                .release_len
                .store(len, Ordering::Release);
            len
        };
        // Kind of arbitrary, but buffering 16 `ScheduledIo`s doesn't seem like much
        const NOTIFY_AFTER: usize = 16;
        if len >= NOTIFY_AFTER {
            self.wake();
        }
        Ok(())
    }
}
