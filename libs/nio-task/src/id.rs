use std::{fmt, future::poll_fn, num::NonZero, task::Poll};

use super::raw::RawTask;
use crate::thin_arc::ThinArc;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct TaskId(pub(crate) NonZero<usize>);

impl TaskId {
    #[inline]
    pub(super) fn new(task: &RawTask) -> TaskId {
        TaskId(unsafe { NonZero::new_unchecked(ThinArc::as_ptr(task).addr()) })
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
