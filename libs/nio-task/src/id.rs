use std::{fmt, future::poll_fn, num::NonZeroUsize, sync::Arc, task::Poll};

use super::raw::RawTask;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct TaskId(pub(crate) NonZeroUsize);

impl TaskId {
    pub(super) fn new(task: &RawTask) -> Self {
        Self(unsafe { NonZeroUsize::new_unchecked(Arc::as_ptr(task).cast::<()>() as usize) })
    }

    #[inline]
    pub fn get(&self) -> usize {
        self.0.get()
    }
}

pub fn id() -> impl Future<Output = TaskId> {
    poll_fn(|cx| {
        let ptr = cx.waker().data();
        let id = unsafe { NonZeroUsize::new_unchecked(ptr as usize) };
        Poll::Ready(TaskId(id))
    })
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
