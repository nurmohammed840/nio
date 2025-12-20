use std::fmt;

use super::{id::Id, raw::RawTask, COMPLETE};

/// An owned permission to abort a spawned task, without awaiting its completion.
///
/// Unlike a [`JoinHandle`], an `AbortHandle` does *not* represent the
/// permission to await the task's completion, only to terminate it.
///
/// The task may be aborted by calling the [`AbortHandle::abort`] method.
/// Dropping an `AbortHandle` releases the permission to terminate the task
/// --- it does *not* abort the task.
///
/// Be aware that tasks spawned using [`spawn_blocking`] cannot be aborted
/// because they are not async. If you call `abort` on a `spawn_blocking` task,
/// then this *will not have any effect*, and the task will continue running
/// normally. The exception is if the task has not started running yet; in that
/// case, calling `abort` may prevent the task from starting.
///
/// [`JoinHandle`]: crate::task::JoinHandle
/// [`spawn_blocking`]: crate::task::spawn_blocking
#[derive(Clone)]
pub struct AbortHandle {
    raw: RawTask,
}

impl AbortHandle {
    pub(super) fn new(raw: RawTask) -> Self {
        Self { raw }
    }

    /// Abort the task associated with the handle.
    ///
    /// Awaiting a cancelled task might complete as usual if the task was
    /// already completed at the time it was cancelled, but most likely it
    /// will fail with a [cancelled] `JoinError`.
    ///
    /// If the task was already cancelled, such as by [`JoinHandle::abort`],
    /// this method will do nothing.
    ///
    /// Be aware that tasks spawned using [`spawn_blocking`] cannot be aborted
    /// because they are not async. If you call `abort` on a `spawn_blocking`
    /// task, then this *will not have any effect*, and the task will continue
    /// running normally. The exception is if the task has not started running
    /// yet; in that case, calling `abort` may prevent the task from starting.
    ///
    /// See also [the module level docs] for more information on cancellation.
    ///
    /// [cancelled]: method@super::error::JoinError::is_cancelled
    /// [`JoinHandle::abort`]: method@super::JoinHandle::abort
    /// [the module level docs]: crate::task#cancellation
    /// [`spawn_blocking`]: crate::task::spawn_blocking
    pub fn abort(&self) {
        unsafe {
            self.raw.abort_task();
        }
    }

    /// Checks if the task associated with this `AbortHandle` has finished.
    ///
    /// Please note that this method can return `false` even if `abort` has been
    /// called on the task. This is because the cancellation process may take
    /// some time, and this method does not return `true` until it has
    /// completed.
    pub fn is_finished(&self) -> bool {
        let state = self.raw.header().state.load();
        state.is(COMPLETE)
    }

    pub(crate) fn id(&self) -> Id {
        Id::new(&self.raw)
    }
}

impl std::panic::UnwindSafe for AbortHandle {}
impl std::panic::RefUnwindSafe for AbortHandle {}

impl fmt::Debug for AbortHandle {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("AbortHandle")
            .field("id", &self.id())
            .finish()
    }
}
