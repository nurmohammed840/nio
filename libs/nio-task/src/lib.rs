#![allow(unsafe_op_in_unsafe_fn)]

mod blocking;
mod error;
mod id;
mod join;
mod raw;
mod state;
mod waker;

use crate::raw::*;

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
    panic::{AssertUnwindSafe, catch_unwind},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
};

pub trait Scheduler: Clone + Send + 'static {
    fn schedule(&self, task: Task);
}

struct RawTaskInner<F: Future, S: Scheduler> {
    header: Header,
    future: UnsafeCell<Fut<F, F::Output>>,
    scheduler: S,
}

unsafe impl<F: Future, S: Scheduler> Sync for RawTaskInner<F, S> {}

pub struct Task {
    raw_task: RawTask,
}

pub enum Status {
    Yielded(Task),
    Pending,
    Complete,
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
            future: UnsafeCell::new(Fut::Future(future)),
            scheduler,
        });
        let join_handle = JoinHandle::new(raw.clone());
        (Self { raw_task: raw }, join_handle)
    }

    #[inline]
    pub fn poll(self) -> Status {
        // Don't increase ref-counter
        let raw_task = unsafe { Arc::from_raw(Arc::as_ptr(&self.raw_task)) };
        // Don't decrease ref-counter
        let waker = ManuallyDrop::new(raw_task.waker());

        // SAFETY: `Task` does not implement `Clone` and we have owned access
        match unsafe { self.raw_task.poll(&waker) } {
            PollStatus::Yield => Status::Yielded(self),
            PollStatus::Pending => Status::Pending,
            PollStatus::Complete => Status::Complete,
        }
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

    /// State transitions:
    ///
    /// ```markdown
    /// NOTIFIED -> RUNNING -> ( SLEEP? -> NOTIFIED -> RUNNING )* -> COMPLETE?
    /// ```
    unsafe fn poll(&self, waker: &Waker) -> PollStatus {
        let is_cancelled = self.header.transition_to_running_and_check_if_cancelled();

        let has_output = catch_unwind(AssertUnwindSafe(|| {
            let poll_result = unsafe {
                let fut = match &mut *self.future.get() {
                    Fut::Future(fut) => Pin::new_unchecked(fut),
                    _ => unreachable!(),
                };
                // Polling may panic, but we catch it in outer layer.
                fut.poll(&mut Context::from_waker(waker))
            };
            let result = match poll_result {
                Poll::Ready(val) => Fut::Output(Ok(val)),
                Poll::Pending if is_cancelled => Fut::Output(Err(JoinError::cancelled())),
                Poll::Pending => return false,
            };
            // Droping `Fut::Future` may also panic, but we catch it in outer layer
            unsafe { *self.future.get() = result }
            true
        }));

        match has_output {
            Ok(false) => return self.header.transition_to_sleep(),
            Ok(true) => {}
            Err(err) => unsafe { (*self.future.get()).set_err_output(JoinError::panic(err)) },
        }
        if !self
            .header
            .transition_to_complete_and_notify_output_if_intrested()
        {
            // Droping `Fut::Output` may panic
            let _ = catch_unwind(AssertUnwindSafe(|| unsafe { (*self.future.get()).drop() }));
        }
        PollStatus::Complete
    }

    unsafe fn schedule(self: Arc<Self>) {
        self.scheduler.clone().schedule(Task { raw_task: self });
    }

    unsafe fn read_output(&self, dst: *mut (), waker: &Waker) {
        if self.header.can_read_output_or_notify_when_readable(waker) {
            *(dst as *mut _) = Poll::Ready(self.take_output());
        }
    }

    unsafe fn drop_join_handler(&self) {
        let is_task_complete = self.header.state.unset_waker_and_interested();
        if is_task_complete {
            // If the task is complete then waker is droped by the executor.
            // We just only need to drop the output.
            let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
                (*self.future.get()).drop();
            }));
        } else {
            *self.header.join_waker.get() = None;
        }
    }

    unsafe fn abort_task(self: Arc<Self>) {
        if self.header.transition_to_abort() {
            self.schedule()
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
            Fut::Output(result) => result,
            _ => panic!("JoinHandle polled after completion"),
        }
    }

    unsafe fn schedule_by_ref(self: &Arc<Self>) {
        self.scheduler.schedule(Task {
            raw_task: self.clone(),
        });
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
            if self.header.transition_to_notified() {
                self.schedule();
            }
        }
    }

    fn wake_by_ref(self: &Arc<Self>) {
        unsafe {
            if self.header.transition_to_notified() {
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
