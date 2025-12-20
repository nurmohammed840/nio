#![allow(unsafe_op_in_unsafe_fn)]

mod abort;
mod blocking;
mod error;
mod id;
mod join;
mod raw;
mod state;
mod waker;

use crate::raw::*;

pub use abort::AbortHandle;
pub use blocking::BlockingTask;
pub use error::JoinError;
pub use id::Id;
pub use join::JoinHandle;

use state::*;
use std::{
    cell::UnsafeCell,
    fmt,
    future::Future,
    mem::ManuallyDrop,
    panic::{self, AssertUnwindSafe},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
};

pub trait Scheduler: Clone + Send + 'static {
    fn schedule(&self, task: Task);
}

struct RawTaskInner<F: Future, S: Scheduler> {
    header: Header,
    future: UnsafeCell<Stage<F, F::Output>>,
    scheduler: S,
}

unsafe impl<F: Future, S: Scheduler> Sync for RawTaskInner<F, S> {}

pub struct Task {
    raw_task: RawTask,
}

impl Task {
    pub fn new<F, S>(future: F, scheduler: S) -> (Self, JoinHandle<F::Output>)
    where
        F: Future + Send + 'static,
        F::Output: Send,
        S: Scheduler,
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
    pub fn process(&mut self) -> bool {
        // Don't increase ref-counter
        let raw_task = unsafe { Arc::from_raw(Arc::as_ptr(&self.raw_task)) };
        // Don't decrease ref-counter
        let waker = ManuallyDrop::new(raw_task.waker());

        // SAFETY: `Task` does not implement `Clone` and we have `&mut` access
        unsafe { self.raw_task.process(&waker) }
    }

    #[inline]
    pub fn schedule(self) {
        unsafe { self.raw_task.schedule() }
    }

    #[inline]
    pub fn id(&self) -> Id {
        Id::new(&self.raw_task)
    }
}

impl<F, S> RawTaskVTable for RawTaskInner<F, S>
where
    F: Future + Send + 'static,
    F::Output: Send,
    S: Scheduler,
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

impl<F, S> RawTaskInner<F, S>
where
    F: Future + Send + 'static,
    F::Output: Send,
    S: Scheduler,
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

impl<F, S> Wake for RawTaskInner<F, S>
where
    F: Future + Send + 'static,
    F::Output: Send,
    S: Scheduler,
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

impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task")
            .field("id", &self.id())
            .field("state", &self.raw_task.header().state.load())
            .finish()
    }
}
