mod abort;
mod blocking;
mod error;
mod id;
mod join;
pub mod state;
mod waker;

pub use abort::AbortHandle;
pub use blocking::BlockingTask;
pub use error::JoinError;
pub use id::Id;
pub use join::JoinHandle;

use crate::scheduler;
use state::*;
use std::{
    cell::UnsafeCell,
    fmt,
    future::Future,
    mem::{self, ManuallyDrop},
    panic::{self, AssertUnwindSafe},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
};

struct RawTaskInner<F: Future> {
    header: Header,
    future: UnsafeCell<Stage<F, F::Output>>,
    scheduler: scheduler::Scheduler,
}

pub struct Header {
    state: State,
    join_waker: UnsafeCell<Option<Waker>>,
}

#[repr(C)] // https://github.com/rust-lang/miri/issues/3780
enum Stage<F, T> {
    Running(F),
    Finished(Result<T, JoinError>),
    Consumed,
}

impl<F, T> Stage<F, T> {
    fn take(&mut self) -> Self {
        mem::replace(self, Self::Consumed)
    }

    fn set_output(&mut self, val: T) {
        *self = Self::Finished(Ok(val));
    }

    fn set_err_output(&mut self, join_error: JoinError) {
        *self = Self::Finished(Err(join_error));
    }
}

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

pub(crate) type RawTask = Arc<dyn RawTaskVTable>;

pub struct Task {
    raw_task: RawTask,
}

impl Task {
    pub(crate) fn new<F>(
        future: F,
        scheduler: scheduler::Scheduler,
    ) -> (Self, JoinHandle<F::Output>)
    where
        F: Future + Send + 'static,
        F::Output: Send,
    {
        let raw = Arc::new(RawTaskInner {
            header: Header::new(),
            future: UnsafeCell::new(Stage::Running(future)),
            scheduler,
        });
        let join_handle = JoinHandle::new(raw.clone());
        (Self { raw_task: raw }, join_handle)
    }

    #[inline]
    pub(crate) fn process(&mut self) -> bool {
        let raw_task = unsafe { Arc::from_raw(Arc::as_ptr(&self.raw_task)) };
        let waker = ManuallyDrop::new(raw_task.waker());
        // SAFETY: `Task` does not implement `Clone` and we have `&mut` access
        unsafe { self.raw_task.process(&waker) }
    }

    #[inline]
    pub fn _reschedule(self) {
        unsafe { self.raw_task.schedule() }
    }

    #[inline]
    pub fn id(&self) -> Id {
        Id::new(&self.raw_task)
    }
}

unsafe impl<F: Future> Sync for RawTaskInner<F> {}

impl<F> RawTaskVTable for RawTaskInner<F>
where
    F: Future + Send + 'static,
    F::Output: Send,
{
    #[inline]
    fn waker(self: Arc<Self>) -> Waker {
        Waker::from(self)
    }

    #[inline]
    fn header(&self) -> &Header {
        &self.header
    }

    unsafe fn process(&self, waker: &Waker) -> bool {
        if self.header.transition_to_running() {
            let action = panic::catch_unwind(AssertUnwindSafe(|| {
                let poll_result = unsafe {
                    let fut = match &mut *self.future.get() {
                        Stage::Running(fut) => Pin::new_unchecked(fut),
                        _ => unreachable!(),
                    };
                    let mut cx = Context::from_waker(waker);
                    fut.poll(&mut cx)
                };
                let stage = match poll_result {
                    Poll::Pending => match self.header.on_pending() {
                        yielded @ PendingAction::Yield(_) => return yielded,
                        PendingAction::AbortOrComplete => {
                            Stage::Finished(Err(JoinError::cancelled()))
                        }
                    },
                    Poll::Ready(val) => Stage::Finished(Ok(val)),
                };
                unsafe { *self.future.get() = stage }
                PendingAction::AbortOrComplete
            }));

            match action {
                Ok(PendingAction::Yield(yielded)) => return yielded,
                Ok(PendingAction::AbortOrComplete) => {}
                Err(panic_on_poll) => unsafe {
                    (*self.future.get()).set_err_output(JoinError::panic(panic_on_poll))
                },
            }
            if !self.header.transition_to_complete() {
                let _ =
                    panic::catch_unwind(AssertUnwindSafe(|| unsafe { self.drop_task_or_output() }));
            }
        }
        false
    }

    unsafe fn read_output(&self, dst: *mut (), waker: &Waker) {
        if self.header.can_read_output(waker) {
            *(dst as *mut _) = Poll::Ready(self.take_output());
        }
    }

    unsafe fn schedule(self: Arc<Self>) {
        self.scheduler.clone().schedule(Task { raw_task: self });
    }

    unsafe fn drop_task_or_output(&self) {
        *self.future.get() = Stage::Consumed
    }

    unsafe fn abort_task(&self) {
        if self.header.transition_to_abort() {
            self.cancel_task();
            if !self.header.transition_to_complete() {
                self.drop_task_or_output();
            }
        }
    }
}

impl<F> RawTaskInner<F>
where
    F: Future + Send + 'static,
    F::Output: Send,
{
    unsafe fn take_output(&self) -> Result<F::Output, JoinError> {
        match (*self.future.get()).take() {
            Stage::Finished(output) => output,
            _ => panic!("JoinHandle polled after completion"),
        }
    }

    unsafe fn schedule_by_ref(self: &Arc<Self>) {
        self.scheduler.schedule(Task {
            raw_task: self.clone(),
        });
    }

    unsafe fn cancel_task(&self) {
        let panic_result = panic::catch_unwind(AssertUnwindSafe(|| self.drop_task_or_output()));
        (*self.future.get()).set_err_output(JoinError::from(panic_result));
    }
}

impl<F> Wake for RawTaskInner<F>
where
    F: Future + Send + 'static,
    F::Output: Send,
{
    fn wake(self: Arc<Self>) {
        unsafe {
            if self.header.transition_to_wake() {
                self.schedule();
            }
        }
    }

    fn wake_by_ref(self: &Arc<Self>) {
        unsafe {
            if self.header.transition_to_wake() {
                self.schedule_by_ref();
            }
        }
    }
}

enum PendingAction {
    Yield(bool),
    AbortOrComplete,
}

impl Header {
    fn new() -> Self {
        Self {
            state: State::new(),
            join_waker: UnsafeCell::new(None),
        }
    }

    fn transition_to_running(&self) -> bool {
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
    fn transition_to_complete(&self) -> bool {
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
    fn on_pending(&self) -> PendingAction {
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

    fn transition_to_wake(&self) -> bool {
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

    fn transition_to_abort(&self) -> bool {
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

    fn can_read_output(&self, waker: &Waker) -> bool {
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

impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task")
            .field("id", &self.id())
            .field("state", &self.raw_task.header().state.load())
            .finish()
    }
}
