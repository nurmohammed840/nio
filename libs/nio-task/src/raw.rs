use crate::{JoinError, state::*, thin_arc::ThinArc};
use std::{cell::UnsafeCell, mem, task::Waker};

pub enum Fut<F, T> {
    Future(F),
    Output(Result<T, JoinError>),
    Droped,
}

pub enum PollStatus {
    Yield,
    Pending,
    Complete,
}

impl<F, T> Fut<F, T> {
    pub fn take(&mut self) -> Self {
        mem::replace(self, Self::Droped)
    }

    pub fn drop(&mut self) {
        *self = Self::Droped;
    }

    pub fn set_output(&mut self, result: Result<T, JoinError>) {
        *self = Self::Output(result);
    }

    pub fn take_output(&mut self) -> Result<T, JoinError> {
        match self.take() {
            Fut::Output(result) => result,
            _ => panic!("JoinHandle polled after completion"),
        }
    }
}

#[repr(C)]
pub struct RawTaskHeader<Data: ?Sized> {
    pub header: Header,
    pub data: Data,
}

unsafe impl<Data> Send for RawTaskHeader<Data> {}
unsafe impl<Data> Sync for RawTaskHeader<Data> {}

pub type RawTask = ThinArc<dyn RawTaskVTable>;

pub trait RawTaskVTable {
    fn waker(&self, raw: RawTask) -> Waker;

    unsafe fn metadata(&self) -> *mut ();

    unsafe fn poll(&self, waker: &Waker) -> PollStatus;
    unsafe fn schedule(&self, raw: RawTask);
    unsafe fn drop_task(&self);

    /// `dst: &mut Poll<Result<Future::Output, JoinError>>`
    unsafe fn read_output(&self, dst: *mut (), waker: &Waker);
    unsafe fn drop_output_from_join_handler(&self);
}

#[repr(C)]
pub struct Header {
    pub state: State,
    pub join_waker: UnsafeCell<Option<Waker>>,
}

impl Header {
    pub fn new() -> Self {
        Self {
            state: State::new(),
            join_waker: UnsafeCell::new(None),
        }
    }

    /// `NOTIFIED -> RUNNING`
    pub fn transition_to_running_and_check_if_cancelled(&self) -> bool {
        let state = self.state.set_running();
        debug_assert!(state.is(NOTIFIED), "invalid task state: {state:?}");
        debug_assert!(!state.has(COMPLETE), "poll after complete: {state:?}");
        state.has(CANCELLED)
    }

    /// Returns [`PollStatus::Yield`] if the future NOTIFIED while in the `RUNNING` state.
    /// for example: `yield_now().await`
    ///
    /// After this call, the executor **MUST NOT** access the future field.
    ///
    /// `RUNNING -> SLEEP`
    pub fn transition_to_sleep(&self) -> PollStatus {
        let state = self.state.set_running_to_sleep();
        if state.is(RUNNING) {
            return PollStatus::Pending;
        }
        debug_assert!(state.is(NOTIFIED), "invalid task state: {state:?}");
        PollStatus::Yield
    }

    /// `(RUNNING | SLEEP) -> NOTIFIED`
    ///
    /// Return `true` if the task is in `SLEEP` state,
    pub fn transition_to_notified(&self) -> bool {
        // Completed task is always in `RUNNING` state, so no additional
        // check is required here.
        self.state.set_notified().is(SLEEP)
    }

    /// `(RUNNING | SLEEP) -> NOTIFIED`
    ///
    /// Return `true` if the task is in `SLEEP` state
    pub fn transition_to_notified_with_cancelled_flag(&self) -> bool {
        self.state.set_notified_with_cancelled_flag().is(SLEEP)
    }

    /// If this function return `false`, then the caller is responsible to drop the output.
    ///
    /// `COMPLETE`
    pub fn transition_to_complete_and_notify_output_if_intrested(&self) -> bool {
        let state = self.state.set_complete();

        debug_assert!(
            state.is(RUNNING) || state.is(NOTIFIED),
            "invalid task state: {state:?}"
        );
        if !state.has(JOIN_INTEREST) {
            // The `JoinHandle` is not interested in the output of this task.
            // It is our responsibility to drop the output.
            return false;
        }
        if state.has(JOIN_WAKER) {
            match unsafe { (*self.join_waker.get()).take() } {
                Some(waker) => waker.wake(),
                None => panic!("waker missing"),
            }
        }
        true
    }

    pub fn can_read_output_or_notify_when_readable(&self, waker: &Waker) -> bool {
        let state = self.state.load();
        debug_assert!(state.has(JOIN_INTEREST));

        if state.has(COMPLETE) {
            return true;
        }
        if !state.has(JOIN_WAKER) {
            // the task is not complete, try storing the provided waker in the task's waker field.
            // SAFETY: `JOIN_WAKER` is not set, see docs of `JOIN_WAKER` flag.
            return unsafe { self.set_join_waker(waker.clone()) };
        }
        // We need to replace it.
        let is_complete = self.state.unset_waker();
        if !is_complete {
            unsafe {
                // Optimization: Avoid storing the waker, if it is the same as the current one.
                let old = (*self.join_waker.get()).as_ref().unwrap();
                if old.will_wake(waker) {
                    return self.state.set_waker();
                }
            }
            // SAFETY: `JOIN_WAKER` unset successfully, we can set the new waker.
            // We have the exclusive access to the waker field.
            unsafe { self.set_join_waker(waker.clone()) };
        }
        is_complete
    }

    /// This function return `Err(..)` If task is COMPLETE.
    unsafe fn set_join_waker(&self, waker: Waker) -> bool {
        // Safety: Only the `JoinHandle` may set the `waker` field. When
        // `JOIN_INTEREST` is **not** set, nothing else will touch the field.
        *self.join_waker.get() = Some(waker);
        let is_complete = self.state.set_waker();
        if is_complete {
            *self.join_waker.get() = None;
        }
        is_complete
    }
}
