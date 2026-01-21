use crate::{AbortHandle, raw::RawTask};

use super::{COMPLETE, error::JoinError, id::TaskId};
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

    #[inline]
    pub fn abort_handle(&self) -> AbortHandle {
        AbortHandle {
            raw: self.raw.clone(),
        }
    }

    #[inline]
    pub fn abort(&self) {
        self.raw.abort_task();
    }

    #[inline]
    pub fn is_finished(&self) -> bool {
        self.raw.header().state.load().has(COMPLETE)
    }

    #[inline]
    pub fn id(&self) -> TaskId {
        TaskId::new(&self.raw)
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut ret: Poll<Result<T, JoinError>> = Poll::Pending;
        unsafe {
            self.raw
                .read_output(&mut ret as *mut _ as *mut (), cx.waker());
        }
        ret
    }
}

impl<T> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        let header = self.raw.header();

        let is_task_complete = header.state.unset_waker_and_interested();
        if is_task_complete {
            // If the task is complete then waker is droped by the executor.
            // We just only need to drop the output.
            unsafe { self.raw.drop_output_from_join_handler() };
        } else {
            unsafe { *header.join_waker.get() = None };
        }
    }
}

impl<T> fmt::Debug for JoinHandle<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("JoinHandle")
            .field("id", &self.id())
            .finish()
    }
}
