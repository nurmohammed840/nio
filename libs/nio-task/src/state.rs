//! State transitions:
//!
//! ```markdown
//! NOTIFIED -> RUNNING -> ( SLEEP? -> NOTIFIED -> RUNNING )* -> COMPLETE?
//! ```

use std::{
    fmt,
    sync::atomic::{
        AtomicUsize,
        Ordering::{AcqRel, Acquire},
    },
};

// The task is currently being run.
pub const RUNNING: usize = 0b00;

/// The task is sleeping, waiting to be woken up for further execution.
pub const SLEEP: usize = 0b01;

/// The task is waiting in the queue, Ready to make progress when polled again.  
///
/// It was woken while in the `RUNNING` state,
/// meaning it should be polled again to make further progress.
pub const NOTIFIED: usize = 0b11;

// ------------- FLAGS -------------

/// The task has been polled and has finished execution.
pub const COMPLETE: usize = 1 << 2;

/// The task has been cancelled.
pub const CANCELLED: usize = 1 << 3;

/// The join handle is still around.
pub const JOIN_INTEREST: usize = 1 << 4;

/// A waker has been set.
///
/// After setting this flag, the caller (who waiting for the task to complete to receive the output)
/// lose access of [`crate::raw::Header::join_waker`] field, until this flag is unset.
///
/// This flag represents ownership of the waker stored in [`crate::raw::Header::join_waker`] field.
pub const JOIN_WAKER: usize = 1 << 5;

pub type UpdateResult = Result<Snapshot, ()>;

pub struct State(AtomicUsize);

impl State {
    pub fn new() -> Self {
        Self(AtomicUsize::new(NOTIFIED | JOIN_INTEREST))
    }

    pub fn load(&self) -> Snapshot {
        Snapshot(self.0.load(Acquire))
    }

    pub fn set_running(&self) -> Snapshot {
        Snapshot(self.0.fetch_and(!0b11, AcqRel))
    }

    pub fn set_running_to_sleep(&self) -> Snapshot {
        Snapshot(self.0.fetch_or(SLEEP, AcqRel))
    }

    pub fn set_notified(&self) -> Snapshot {
        Snapshot(self.0.fetch_or(NOTIFIED, AcqRel))
    }

    pub fn set_notified_with_cancelled_flag(&self) -> Snapshot {
        Snapshot(self.0.fetch_or(NOTIFIED | CANCELLED, AcqRel))
    }

    pub fn set_complete(&self) -> Snapshot {
        Snapshot(self.0.fetch_or(COMPLETE, AcqRel))
    }

    pub fn fetch_update<E>(
        &self,
        mut f: impl FnMut(Snapshot) -> Result<Snapshot, E>,
    ) -> Result<Snapshot, E> {
        let mut prev = self.load();
        loop {
            let next = f(prev)?;
            match self
                .0
                .compare_exchange_weak(prev.0, next.0, AcqRel, Acquire)
            {
                Ok(v) => return Ok(Snapshot(v)),
                Err(next_prev) => prev = Snapshot(next_prev),
            }
        }
    }

    /// Return `true` if the task is `COMPLETE`
    pub fn unset_waker_and_interested(&self) -> bool {
        self.fetch_update::<()>(|state| {
            debug_assert!(state.has(JOIN_INTEREST));

            if state.has(COMPLETE) {
                return Err(());
            }
            Ok(state.remove(JOIN_INTEREST).remove(JOIN_WAKER))
        })
        .is_err()
    }

    pub fn set_waker(&self) -> UpdateResult {
        self.fetch_update(|state| {
            debug_assert!(!state.has(JOIN_WAKER));

            if state.has(COMPLETE) {
                return Err(());
            }
            Ok(state.with(JOIN_WAKER))
        })
    }

    pub fn unset_waker(&self) -> UpdateResult {
        self.fetch_update(|state| {
            debug_assert!(state.has(JOIN_WAKER));

            if state.has(COMPLETE) {
                return Err(());
            }
            Ok(state.remove(JOIN_WAKER))
        })
    }
}

/// Current state value.
#[derive(Copy, Clone)]
pub struct Snapshot(usize);

impl Snapshot {
    pub fn is(&self, state: usize) -> bool {
        self.0 & 0b_11 == state
    }

    pub fn has(&self, flag: usize) -> bool {
        self.0 & flag == flag
    }

    pub fn with(&self, flag: usize) -> Snapshot {
        Self(self.0 | flag)
    }

    fn remove(&self, flag: usize) -> Snapshot {
        Self(self.0 & !flag)
    }
}

impl fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Snapshot")
            .field(
                "state",
                &match self.0 & 0b_11 {
                    NOTIFIED => "NOTIFIED",
                    RUNNING => "RUNNING",
                    SLEEP => "SLEEP",
                    _ => "",
                },
            )
            .field("COMPLETE", &self.has(COMPLETE))
            .field("JOIN_INTEREST", &self.has(JOIN_INTEREST))
            .field("JOIN_WAKER", &self.has(JOIN_WAKER))
            .finish()
    }
}
