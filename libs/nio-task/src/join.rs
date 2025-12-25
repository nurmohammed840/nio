use crate::raw::RawTask;

use super::{COMPLETE, error::JoinError, id::Id};
use std::{
    fmt,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

pub struct JoinHandle<T> {
    raw: RawTask,
    _p: PhantomData<T>,
}
unsafe impl<T: Send> Send for JoinHandle<T> {}
unsafe impl<T: Send> Sync for JoinHandle<T> {}

impl<T> std::panic::UnwindSafe for JoinHandle<T> {}
impl<T> std::panic::RefUnwindSafe for JoinHandle<T> {}

impl<T> JoinHandle<T> {
    pub(super) fn new(raw: RawTask) -> JoinHandle<T> {
        JoinHandle {
            raw,
            _p: PhantomData,
        }
    }

    pub fn abort(&self) {
        unsafe { self.raw.clone().abort_task() };
    }

    pub fn is_finished(&self) -> bool {
        self.raw.header().state.load().has(COMPLETE)
    }

    pub fn id(&self) -> Id {
        Id::new(&self.raw)
    }
}

impl<T> Unpin for JoinHandle<T> {}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut ret = Poll::Pending;
        unsafe {
            self.raw
                .read_output(&mut ret as *mut _ as *mut (), cx.waker());
        }

        ret
    }
}

impl<T> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        unsafe { self.raw.drop_join_handler() };
    }
}

impl<T> fmt::Debug for JoinHandle<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("JoinHandle")
            .field("id", &self.id())
            .finish()
    }
}
