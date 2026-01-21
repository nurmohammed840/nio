#![doc = include_str!("../README.md")]
#![allow(unsafe_op_in_unsafe_fn)]

mod abort;
mod blocking;
mod error;
mod id;
mod join;
mod raw;
mod state;
mod task;
mod thin_arc;
mod waker;

use crate::{raw::*, thin_arc::ThinArc};

pub use abort::AbortHandle;
pub use blocking::BlockingTask;
pub use error::JoinError;
pub use id::{TaskId, id};
pub use join::JoinHandle;

use state::*;
use std::{
    cell::UnsafeCell,
    fmt::{self, Debug},
    future::Future,
    marker::PhantomData,
    mem::ManuallyDrop,
};

pub trait Scheduler<M = ()>: 'static {
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

pub struct Metadata<M = ()> {
    raw: RawTask,
    _meta: PhantomData<M>,
}

impl<M: Debug> Debug for Metadata<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

impl<M> Metadata<M> {
    pub fn id(&self) -> TaskId {
        TaskId::new(&self.raw)
    }

    pub fn get(&self) -> &M {
        unsafe { &*self.raw.metadata().cast::<M>() }
    }

    pub fn get_mut(&mut self) -> &mut M {
        unsafe { &mut *self.raw.metadata().cast::<M>() }
    }
}

#[derive(Debug)]
pub enum Status<M> {
    Yielded(Task<M>),
    Pending,
    Complete(Metadata<M>),
}

impl Task {
    pub fn new<F, S>(future: F, scheduler: S) -> (Task, JoinHandle<F::Output>)
    where
        S: Scheduler<()> + Send,
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        unsafe { Self::new_unchecked((), future, scheduler) }
    }

    pub fn new_local<F, S>(future: F, scheduler: S) -> (Task, JoinHandle<F::Output>)
    where
        S: Scheduler<()> + Send,
        F: Future + 'static,
        F::Output: 'static,
    {
        Self::new_local_with((), future, scheduler)
    }
}

impl<M> Task<M> {
    pub(crate) fn from_raw(raw: RawTask) -> Self {
        Self {
            raw: Some(raw),
            _meta: PhantomData,
        }
    }

    pub fn metadata(&self) -> &M {
        unsafe { &*self.raw.as_ref().unwrap_unchecked().metadata().cast() }
    }

    pub fn metadata_mut(&mut self) -> &mut M {
        unsafe { &mut *self.raw.as_ref().unwrap_unchecked().metadata().cast() }
    }

    pub unsafe fn new_unchecked<F, S>(
        meta: M,
        future: F,
        scheduler: S,
    ) -> (Task<M>, JoinHandle<F::Output>)
    where
        M: 'static,
        S: Scheduler<M>,
        F: Future + 'static,
    {
        let (raw, join) = ThinArc::new(Box::new(RawTaskHeader {
            header: Header::new(),
            data: task::RawTaskInner {
                future: UnsafeCell::new(Fut::Future(future)),
                meta: UnsafeCell::new(meta),
                scheduler,
            },
        }));
        (Task::from_raw(raw), JoinHandle::new(join))
    }

    pub fn new_with<F, S>(meta: M, future: F, scheduler: S) -> (Task<M>, JoinHandle<F::Output>)
    where
        M: 'static + Send,
        S: Scheduler<M> + Send,
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        unsafe { Self::new_unchecked(meta, future, scheduler) }
    }

    pub fn new_local_with<F, S>(
        meta: M,
        future: F,
        scheduler: S,
    ) -> (Task<M>, JoinHandle<F::Output>)
    where
        M: 'static + Send,
        S: Scheduler<M> + Send,
        F: Future + 'static,
        F::Output: 'static,
    {
        use std::{
            mem::ManuallyDrop,
            pin::Pin,
            task::{Context, Poll},
            thread::{self, ThreadId},
        };

        #[inline]
        fn thread_id() -> ThreadId {
            std::thread_local! {
                static ID: ThreadId = thread::current().id();
            }
            ID.try_with(|id| *id)
                .unwrap_or_else(|_| thread::current().id())
        }

        struct Checked<F> {
            id: ThreadId,
            inner: ManuallyDrop<F>,
        }

        impl<F> Drop for Checked<F> {
            fn drop(&mut self) {
                assert!(
                    self.id == thread_id(),
                    "local task dropped by a thread that didn't spawn it"
                );
                unsafe {
                    ManuallyDrop::drop(&mut self.inner);
                }
            }
        }

        impl<F: Future> Future for Checked<F> {
            type Output = F::Output;
            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                unsafe {
                    let me = self.get_unchecked_mut();
                    assert!(
                        me.id == thread_id(),
                        "local task polled by a thread that didn't spawn it"
                    );
                    Pin::new_unchecked(&mut *me.inner).poll(cx)
                }
            }
        }

        let future = Checked {
            id: thread_id(),
            inner: ManuallyDrop::new(future),
        };

        unsafe { Self::new_unchecked(meta, future, scheduler) }
    }

    #[inline]
    pub fn poll(mut self) -> Status<M> {
        let raw = unsafe { self.raw.take().unwrap_unchecked() };
        // Don't increase ref-counter
        let waker = raw.clone_without_ref_inc();
        // Don't decrease ref-counter
        let waker = ManuallyDrop::new(raw.waker(waker));

        // SAFETY: `Task` does not implement `Clone` and we have owned access
        match unsafe { raw.poll(&waker) } {
            PollStatus::Yield => Status::Yielded(Task::from_raw(raw)),
            PollStatus::Pending => Status::Pending,
            PollStatus::Complete => Status::Complete(Metadata {
                raw,
                _meta: PhantomData,
            }),
        }
    }

    #[inline]
    pub fn schedule(mut self) {
        unsafe {
            let raw = self.raw.take().unwrap_unchecked();
            raw.schedule(raw.clone());
        }
    }

    #[inline]
    pub fn id(&self) -> TaskId {
        TaskId::new(unsafe { self.raw.as_ref().unwrap_unchecked() })
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
