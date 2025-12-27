#![doc = include_str!("../README.md")]
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
    fmt::{self, Debug},
    future::Future,
    marker::PhantomData,
    mem::ManuallyDrop,
    panic::{AssertUnwindSafe, catch_unwind},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
};

pub trait Scheduler<M>: 'static {
    fn schedule(&self, task: Task<M>);
}

impl<F, M> Scheduler<M> for F
where
    F: Fn(Task<M>) + 'static,
{
    fn schedule(&self, runnable: Task<M>) {
        self(runnable)
    }
}

struct RawTaskInner<F: Future, S: Scheduler<M>, M> {
    header: Header,
    future: UnsafeCell<Fut<F, F::Output>>,
    meta: UnsafeCell<M>,
    scheduler: S,
}

unsafe impl<F: Future, S: Scheduler<M>, M> Send for RawTaskInner<F, S, M> {}
unsafe impl<F: Future, S: Scheduler<M>, M> Sync for RawTaskInner<F, S, M> {}

pub struct Task<M = ()> {
    raw: Option<RawTask>,
    _meta: PhantomData<M>,
}

unsafe impl<M> Send for Task<M> {}
unsafe impl<M> Sync for Task<M> {}

impl<M> std::panic::UnwindSafe for Task<M> {}
impl<M> std::panic::RefUnwindSafe for Task<M> {}

impl<M> Drop for Task<M> {
    fn drop(&mut self) {
        if let Some(raw) = self.raw.take() {
            unsafe { raw.drop_task() };
        }
    }
}

// ------------------------------------------------

pub struct Metadata<M>(Task<M>);

impl<M> Metadata<M> {
    pub fn get(&self) -> &M {
        self.0.metadata()
    }

    pub fn get_mut(&mut self) -> &mut M {
        self.0.metadata_mut()
    }
}

pub enum Status<M> {
    Yielded(Task<M>),
    Pending,
    Complete(Metadata<M>),
}

impl Task {
    pub fn new<F, S>(future: F, scheduler: S) -> (Self, JoinHandle<F::Output>)
    where
        F: Future + Send + 'static,
        F::Output: Send,
        S: Scheduler<()>,
    {
        Self::new_with((), future, scheduler)
    }

    pub fn new_local<F, S>(future: F, scheduler: S) -> (Self, JoinHandle<F::Output>)
    where
        F: Future + 'static,
        F::Output: 'static,
        S: Scheduler<()>,
    {
        Self::new_local_with((), future, scheduler)
    }
}

impl<M> Task<M> {
    pub fn metadata(&self) -> &M {
        unsafe {
            &*(self.raw.as_ref().unwrap_unchecked())
                .metadata()
                .cast::<M>()
        }
    }

    pub fn metadata_mut(&mut self) -> &mut M {
        unsafe {
            &mut *(self.raw.as_ref().unwrap_unchecked())
                .metadata()
                .cast::<M>()
        }
    }

    pub fn new_with<F, S>(meta: M, future: F, scheduler: S) -> (Self, JoinHandle<F::Output>)
    where
        M: 'static + Send,
        F: Future + Send + 'static,
        F::Output: Send,
        S: Scheduler<M>,
    {
        let raw = Arc::new(RawTaskInner {
            header: Header::new(),
            future: UnsafeCell::new(Fut::Future(future)),
            meta: UnsafeCell::new(meta),
            scheduler,
        });
        let join_handle = JoinHandle::new(raw.clone());
        (
            Self {
                raw: Some(raw),
                _meta: PhantomData,
            },
            join_handle,
        )
    }

    pub fn new_local_with<F, S>(meta: M, future: F, scheduler: S) -> (Self, JoinHandle<F::Output>)
    where
        M: 'static + Send,
        F: Future + 'static,
        F::Output: 'static,
        S: Scheduler<M>,
    {
        let raw = Arc::new(RawTaskInner {
            header: Header::new(),
            future: UnsafeCell::new(Fut::Future(future)),
            meta: UnsafeCell::new(meta),
            scheduler,
        });
        let join_handle = JoinHandle::new(raw.clone());
        (
            Self {
                raw: Some(raw),
                _meta: PhantomData,
            },
            join_handle,
        )
    }

    #[inline]
    pub fn poll(mut self) -> Status<M> {
        let inner = unsafe { self.raw.take().unwrap_unchecked() };
        // Don't increase ref-counter
        let raw = unsafe { Arc::from_raw(Arc::as_ptr(&inner)) };
        // Don't decrease ref-counter
        let waker = ManuallyDrop::new(raw.waker());

        // SAFETY: `Task` does not implement `Clone` and we have owned access
        match unsafe { inner.poll(&waker) } {
            PollStatus::Yield => Status::Yielded(Self {
                raw: Some(inner),
                _meta: PhantomData,
            }),
            PollStatus::Pending => Status::Pending,
            PollStatus::Complete => Status::Complete(Metadata(self)),
        }
    }

    #[inline]
    pub fn schedule(mut self) {
        unsafe { self.raw.take().unwrap_unchecked().schedule() }
    }

    #[inline]
    pub fn id(&self) -> Id {
        Id::new(unsafe { self.raw.as_ref().unwrap_unchecked() })
    }
}

impl<F, S, M> RawTaskVTable for RawTaskInner<F, S, M>
where
    M: 'static,
    F: Future + 'static,
    S: Scheduler<M>,
{
    #[inline]
    fn waker(self: Arc<Self>) -> Waker {
        Waker::from(self)
    }

    #[inline]
    fn header(&self) -> &Header {
        &self.header
    }

    unsafe fn metadata(&self) -> *mut () {
        self.meta.get().cast()
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
                Poll::Ready(val) => Ok(val),
                Poll::Pending if is_cancelled => Err(JoinError::cancelled()),
                Poll::Pending => return false,
            };
            // Droping `Fut::Future` may also panic, but we catch it in outer layer
            unsafe {
                (*self.future.get()).set_output(result);
            }
            true
        }));

        match has_output {
            Ok(false) => return self.header.transition_to_sleep(),
            Ok(true) => {}
            Err(err) => unsafe { (*self.future.get()).set_output(Err(JoinError::panic(err))) },
        }
        if !self
            .header
            .transition_to_complete_and_notify_output_if_intrested()
        {
            // Receiver is not interested in the output, So we can drop it.
            // Droping `Fut::Output` may panic
            let _ = catch_unwind(AssertUnwindSafe(|| unsafe { (*self.future.get()).drop() }));
        }
        PollStatus::Complete
    }

    unsafe fn schedule(self: Arc<Self>) {
        self.scheduler.schedule(Task {
            raw: Some(self.clone()),
            _meta: PhantomData,
        });
    }

    unsafe fn drop_task(self: Arc<Self>) {
        // TODO: Also drop task metadata
        unsafe { (*self.future.get()).drop() }
    }

    unsafe fn abort_task(self: Arc<Self>) {
        if self.header.transition_to_abort() {
            self.schedule()
        }
    }

    unsafe fn read_output(&self, dst: *mut (), waker: &Waker) {
        if self.header.can_read_output_or_notify_when_readable(waker) {
            *(dst as *mut _) = Poll::Ready((*self.future.get()).take_output());
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
}

impl<F, S, M> RawTaskInner<F, S, M>
where
    M: 'static,
    F: Future + 'static,
    S: Scheduler<M>,
{
    unsafe fn schedule_by_ref(self: &Arc<Self>) {
        self.scheduler.schedule(Task {
            raw: Some(self.clone()),
            _meta: PhantomData,
        });
    }
}

impl<F, S, M> Wake for RawTaskInner<F, S, M>
where
    M: 'static,
    F: Future + 'static,
    S: Scheduler<M>,
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

impl<M: Debug> fmt::Debug for Task<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task")
            .field("id", &self.id())
            .field("state", &self.raw.as_ref().unwrap().header().state.load())
            .field("metadata", self.metadata())
            .finish()
    }
}
