use std::{fmt, future::poll_fn, num::NonZero, sync::Arc, task::Poll};

use super::raw::RawTask;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct TaskId(pub(crate) NonZero<usize>);

impl TaskId {
    pub(super) fn new(task: &RawTask) -> TaskId {
        TaskId(unsafe { NonZero::new_unchecked(Arc::as_ptr(task).addr()) })
    }

    #[inline]
    pub fn get(&self) -> NonZero<usize> {
        self.0
    }
}

pub fn id() -> impl Future<Output = TaskId> {
    poll_fn(|cx| {
        Poll::Ready(TaskId(unsafe {
            NonZero::new_unchecked(cx.waker().data().addr())
        }))
    })
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
