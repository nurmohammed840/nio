use futures::task::AtomicWaker;

use crate::io::interest::Interest;
use crate::io::ready::Ready;

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::{AcqRel, Acquire};
use std::sync::Arc;
use std::task::{Context, Poll};

use super::utils::bit;

// # This struct should be cache padded to avoid false sharing. The cache padding rules are copied
#[cfg_attr(
    any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "powerpc64",
    ),
    repr(align(128))
)]
#[cfg_attr(
    any(
        target_arch = "arm",
        target_arch = "mips",
        target_arch = "mips32r6",
        target_arch = "mips64",
        target_arch = "mips64r6",
        target_arch = "sparc",
        target_arch = "hexagon",
    ),
    repr(align(32))
)]
#[cfg_attr(target_arch = "m68k", repr(align(16)))]
#[cfg_attr(target_arch = "s390x", repr(align(256)))]
#[cfg_attr(
    not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "powerpc64",
        target_arch = "arm",
        target_arch = "mips",
        target_arch = "mips32r6",
        target_arch = "mips64",
        target_arch = "mips64r6",
        target_arch = "sparc",
        target_arch = "hexagon",
        target_arch = "m68k",
        target_arch = "s390x",
    )),
    repr(align(64))
)]
pub struct ScheduledIo {
    /// Packs the resource's readiness and I/O driver latest tick.
    readiness: AtomicUsize,
    /// Waker used for `AsyncRead`.
    reader: AtomicWaker,
    writer: AtomicWaker,
}

#[derive(Debug)]
pub struct ReadyEvent {
    pub ready: Ready,
    tick: u8,
}

pub enum Tick {
    Set,
    Clear(u8),
}

// The `ScheduledIo::readiness` (`AtomicUsize`) is packed full of goodness.
//
// | driver tick | readiness |
// |-------------+-----------|
// |  15 bits    +   16 bits |

const READINESS: bit::Pack = bit::Pack::least_significant(16);
const TICK: bit::Pack = READINESS.then(15);

// ===== impl ScheduledIo =====

impl Default for ScheduledIo {
    fn default() -> ScheduledIo {
        ScheduledIo {
            readiness: AtomicUsize::new(0),
            reader: AtomicWaker::new(),
            writer: AtomicWaker::new(),
        }
    }
}

impl ScheduledIo {
    #[inline]
    pub fn into_token(self: &Arc<Self>) -> usize {
        Arc::as_ptr(self) as usize
    }

    #[inline]
    pub fn from_token(token: usize) -> *const ScheduledIo {
        token as *const ScheduledIo
    }

    /// Polls for readiness events in a given direction.
    ///
    /// These are to support `AsyncRead` and `AsyncWrite` polling methods,
    /// which cannot use the `async fn` version. This uses reserved reader
    /// and writer slots.
    pub fn poll_readiness(&self, cx: &mut Context<'_>, interest: Interest) -> Poll<ReadyEvent> {
        let curr = self.readiness.load(Acquire);
        let ready = Ready::from_usize(curr).intersection(interest);

        if !ready.is_empty() {
            return Poll::Ready(ReadyEvent {
                tick: TICK.unpack(curr) as u8,
                ready,
            });
        }
        if interest == Interest::READABLE {
            self.reader.register(cx.waker());
        } else {
            debug_assert_eq!(interest, Interest::WRITABLE);
            self.writer.register(cx.waker());
        };
        Poll::Pending
    }

    pub fn clear_readiness(&self, event: ReadyEvent) {
        // This consumes the current readiness state **except** for closed
        // states. Closed states are excluded because they are final states.
        let mask_no_closed = event.ready - Ready::READ_CLOSED - Ready::WRITE_CLOSED;
        self.set_readiness(Tick::Clear(event.tick), |curr| curr - mask_no_closed);
    }

    /// Sets the readiness on this `ScheduledIo` by invoking the given closure on
    /// the current value, returning the previous readiness value.
    ///
    /// # Arguments
    /// - `tick`: whether setting the tick or trying to clear readiness for a
    ///    specific tick.
    /// - `f`: a closure returning a new readiness value given the previous
    ///   readiness.
    pub fn set_readiness(&self, tick_op: Tick, f: impl Fn(Ready) -> Ready) {
        let _ = self.readiness.fetch_update(AcqRel, Acquire, |curr| {
            const MAX_TICK: usize = TICK.max_value() + 1; // same as `1 << 15`
            let tick = TICK.unpack(curr);
            let new_tick = match tick_op {
                // Trying to clear readiness with an old event!
                Tick::Clear(t) if tick as u8 != t => return None,
                Tick::Clear(t) => t as usize,
                Tick::Set => tick.wrapping_add(1) % MAX_TICK,
            };
            Some(TICK.pack(new_tick, f(Ready::from_usize(curr)).as_usize()))
        });
    }

    /// Notifies all pending waiters that have registered interest in `ready`.
    ///
    /// There may be many waiters to notify. Waking the pending task **must** be
    /// done from outside of the lock otherwise there is a potential for a
    /// deadlock.
    ///
    /// A stack array of wakers is created and filled with wakers to notify, the
    /// lock is released, and the wakers are notified. Because there may be more
    /// than 32 wakers to notify, if the stack array fills up, the lock is
    /// released, the array is cleared, and the iteration continues.
    pub fn wake(&self, ready: Ready) {
        if ready.is_readable() {
            self.reader.wake();
        }
        if ready.is_writable() {
            self.writer.wake();
        }
    }

    pub fn ready_event(&self, interest: Interest) -> ReadyEvent {
        let curr = self.readiness.load(Acquire);
        ReadyEvent {
            tick: TICK.unpack(curr) as u8,
            ready: interest.mask() & Ready::from_usize(curr),
        }
    }

    pub fn drop_wakers(&self) {
        self.reader.take();
        self.writer.take();
    }
}

impl Drop for ScheduledIo {
    fn drop(&mut self) {
        self.wake(Ready::ALL);
    }
}
