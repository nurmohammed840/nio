use crate::{JoinError, state::*};
use std::{cell::UnsafeCell, mem, sync::Arc, task::Waker};

#[repr(C)] // https://github.com/rust-lang/miri/issues/3780
pub enum Stage<F, T> {
    Running(F),
    Finished(Result<T, JoinError>),
    Consumed,
}

pub enum PendingAction {
    Yield(bool),
    AbortOrComplete,
}

impl<F, T> Stage<F, T> {
    pub(crate) fn take(&mut self) -> Self {
        mem::replace(self, Self::Consumed)
    }

    pub(crate) fn set_output(&mut self, val: T) {
        *self = Self::Finished(Ok(val));
    }

    pub(crate) fn set_err_output(&mut self, join_error: JoinError) {
        *self = Self::Finished(Err(join_error));
    }
}

pub(crate) type RawTask = Arc<dyn RawTaskVTable>;

pub(crate) trait RawTaskVTable: Send + Sync {
    fn header(&self) -> &Header;
    fn waker(self: Arc<Self>) -> Waker;

    unsafe fn drop_task_or_output(&self);
    unsafe fn process(&self, waker: &Waker) -> bool;
    unsafe fn abort_task(&self);

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

    pub fn transition_to_running(&self) -> bool {
        self.state
            .fetch_update(|snapshot| {
                let state = snapshot.get();
                if state == NOTIFIED {
                    return Some(snapshot.set(RUNNING));
                }
                // `state == COMPLETE` can occur while aborting a task if the task
                // was in the `NOTIFIED` state.
                debug_assert!(
                    state == ABORT || state == COMPLETE,
                    "invalid task state: {snapshot:?}"
                );
                None
            })
            .is_ok()
    }

    /// If this function return `false`, then the caller is responsible to drop the output.
    pub fn transition_to_complete(&self) -> bool {
        let snapshot = self.state.set_complete();
        let state = snapshot.get();

        debug_assert!(
            state == RUNNING || state == YIELD || state == ABORT ||
            // Blocking task doen't transition to `RUNNING` state
            state == NOTIFIED,
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

    /// Returns `true` if the future yielded execution back
    /// to the executor, via `yield_now().await`
    pub fn on_pending(&self) -> PendingAction {
        let op = self.state.fetch_update(|snapshot| {
            let state = snapshot.get();
            if state == RUNNING {
                return Some(snapshot.set(SLEEP));
            }
            if state == YIELD {
                return Some(snapshot.set(NOTIFIED));
            }
            // `SLEEP`, `NOTIFIED`, `COMPLETE` state are not possible
            debug_assert!(state == ABORT, "invalid task state: {snapshot:?}");
            None
        });
        match op {
            Ok(state) => PendingAction::Yield(state.is(YIELD)),
            Err(_) => PendingAction::AbortOrComplete,
        }
    }

    pub fn transition_to_wake(&self) -> bool {
        let op = self.state.fetch_update(|snapshot| {
            let state = snapshot.get();
            if state == SLEEP {
                return Some(snapshot.set(NOTIFIED));
            }
            if state == RUNNING {
                return Some(snapshot.set(YIELD));
            }
            debug_assert!(
                state == COMPLETE || state == ABORT || state == YIELD || state == NOTIFIED,
                "invalid task state: {snapshot:?}"
            );
            None
        });
        op.is_ok_and(|s| s.is(SLEEP))
    }

    pub fn transition_to_abort(&self) -> bool {
        let op = self.state.fetch_update(|snapshot| {
            let state = snapshot.get();
            if state == ABORT || state == COMPLETE {
                return None;
            }
            Some(snapshot.set(ABORT))
        });
        op.is_ok_and(|snapshot| {
            let state = snapshot.get();
            state == NOTIFIED || state == SLEEP
        })
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
            Err(_s) => {
                debug_assert!(_s.is(COMPLETE));
                true
            }
        }
    }

    /// This function return `Err(..)` If task is COMPLETE.
    fn set_join_waker(&self, waker: Waker) -> Result<Snapshot, Snapshot> {
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
