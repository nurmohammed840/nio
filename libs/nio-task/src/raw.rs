use crate::{JoinError, state::*};
use std::{cell::UnsafeCell, mem, sync::Arc, task::Waker};

#[repr(C)] // https://github.com/rust-lang/miri/issues/3780
pub enum Fut<F, T> {
    Running(F),
    Result(Result<T, JoinError>),
    Droped,
}

pub enum PendingState {
    /// `false` if task is sleeping
    Yielded(bool),
    AbortOrComplete,
}

impl<F, T> Fut<F, T> {
    pub(crate) fn take(&mut self) -> Self {
        mem::replace(self, Self::Droped)
    }

    pub(crate) fn set_output(&mut self, val: T) {
        *self = Self::Result(Ok(val));
    }

    pub(crate) fn set_err_output(&mut self, join_error: JoinError) {
        *self = Self::Result(Err(join_error));
    }
}

pub(crate) type RawTask = Arc<dyn RawTaskVTable>;

pub(crate) trait RawTaskVTable: Send + Sync {
    fn header(&self) -> &Header;
    fn waker(self: Arc<Self>) -> Waker;

    unsafe fn drop_task_or_output(&self);
    unsafe fn poll(&self, waker: &Waker) -> bool;
    unsafe fn abort_task(self: Arc<Self>);

    /// `dst: &mut Poll<Result<Future::Output, JoinError>>`
    unsafe fn read_output(&self, dst: *mut (), waker: &Waker);
    unsafe fn schedule(self: Arc<Self>);
}

pub(crate) struct Header {
    pub(crate) state: State,
    pub(crate) join_waker: UnsafeCell<Option<Waker>>,
}

impl Header {
    pub fn new() -> Self {
        Self {
            state: State::new(),
            join_waker: UnsafeCell::new(None),
        }
    }

    /// `NOTIFIED -> RUNNING`
    pub fn transition_to_running(&self) -> bool {
        self.state
            .fetch_update(|snapshot| {
                let state = snapshot.get();
                if state == NOTIFIED {
                    return Ok(snapshot.set(RUNNING));
                }
                if cfg!(debug_assertions) {
                    panic!("invalid task state: {snapshot:?}");
                }
                Err(())
            })
            .is_ok()
    }

    /// If this function return `false`, then the caller is responsible to drop the output.
    ///
    /// `COMPLETE`
    pub fn transition_to_complete_and_notify_output_if_intrested(&self) -> bool {
        let snapshot = self.state.set_complete();
        let state = snapshot.get();

        debug_assert!(
            state == RUNNING || state == NOTIFIED,
            "invalid task state: {snapshot:?}"
        );
        if !snapshot.has(JOIN_INTEREST) {
            // The `JoinHandle` is not interested in the output of this task.
            // It is our responsibility to drop the output.
            return false;
        }
        if snapshot.has(JOIN_WAKER) {
            match unsafe { (*self.join_waker.get()).as_ref() } {
                Some(waker) => waker.wake_by_ref(),
                None => panic!("waker missing"),
            }
        }
        true
    }

    /// Returns `PendingState::Yielded(true)` if the future waken (`NOTIFIED`)
    /// while in the `RUNNING` state. via `yield_now().await`
    ///
    /// `RUNNING -> SLEEP`
    pub fn transition_to_sleep(&self) -> PendingState {
        let op = self.state.fetch_update(|snapshot| {
            let state = snapshot.get();
            if state == RUNNING {
                return Ok(snapshot.set(SLEEP));
            }

            if snapshot.has(CANCELLED) {
                return Err(PendingState::AbortOrComplete);
            }

            debug_assert!(state == NOTIFIED, "invalid task state: {snapshot:?}");
            Err(PendingState::Yielded(true))
        });
        match op {
            Ok(_) => PendingState::Yielded(false),
            Err(state) => state,
        }
    }

    /// `(RUNNING | SLEEP) -> NOTIFIED`
    ///
    /// Return `true` if the task is in `SLEEP` state
    pub fn transition_to_wake(&self) -> bool {
        let op = self.state.fetch_update(|snapshot| {
            let state = snapshot.get();
            if state == RUNNING || state == SLEEP {
                return Ok(snapshot.set(NOTIFIED));
            }
            Err(())
        });
        op.is_ok_and(|s| s.is(SLEEP))
    }

    /// NOTIFIED, with
    pub fn transition_to_abort(&self) -> bool {
        let op = self.state.fetch_update(|snapshot| {
            let state = snapshot.get();
            if state == RUNNING || state == SLEEP {
                return Ok(snapshot.with(CANCELLED).set(NOTIFIED));
            }
            Err(())
        });
        op.is_ok_and(|s| s.is(SLEEP))
    }

    pub fn can_read_output(&self, waker: &Waker) -> bool {
        let snapshot = self.state.load();
        debug_assert!(snapshot.has(JOIN_INTEREST));

        if snapshot.is(COMPLETE) {
            return true;
        }
        // If the task is not complete, try storing the provided waker in the task's waker field.
        let res = if snapshot.has(JOIN_WAKER) {
            unsafe {
                let join_waker = (*self.join_waker.get()).as_ref().unwrap();
                if join_waker.will_wake(waker) {
                    return false;
                }
            }
            self.state
                .unset_waker()
                .and_then(|_| self.set_join_waker(waker.clone()))
        } else {
            self.set_join_waker(waker.clone())
        };

        match res {
            Ok(_) => false,
            Err(_s) => true,
        }
    }

    /// This function return `Err(..)` If task is COMPLETE.
    fn set_join_waker(&self, waker: Waker) -> Result<Snapshot, ()> {
        // Safety: Only the `JoinHandle` may set the `waker` field. When
        // `JOIN_INTEREST` is **not** set, nothing else will touch the field.
        unsafe { *self.join_waker.get() = Some(waker) };
        let res = self.state.set_join_waker();
        // If the state could not be updated, then clear the join waker
        if res.is_err() {
            unsafe { *self.join_waker.get() = None };
        }
        res
    }
}
