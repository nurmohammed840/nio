use crate::raw::RawTask;

use super::{COMPLETE, abort::AbortHandle, error::JoinError, id::Id};
use std::{
    fmt,
    future::Future,
    marker::PhantomData,
    panic,
    pin::Pin,
    task::{Context, Poll},
};

/// An owned permission to join on a task (await its termination).
///
/// This can be thought of as the equivalent of [`std::thread::JoinHandle`]
/// for a Nio task rather than a thread. Note that the background task
/// associated with this `JoinHandle` started running immediately when you
/// called spawn, even if you have not yet awaited the `JoinHandle`.
///
/// A `JoinHandle` *detaches* the associated task when it is dropped, which
/// means that there is no longer any handle to the task, and no way to `join`
/// on it.
///
/// This `struct` is created by the [`task::spawn`] and [`task::spawn_blocking`]
/// functions.
///
/// # Cancel safety
///
/// The `&mut JoinHandle<T>` type is cancel safe. If it is used as the event
/// in a `nio::select!` statement and some other branch completes first,
/// then it is guaranteed that the output of the task is not lost.
///
/// If a `JoinHandle` is dropped, then the task continues running in the
/// background and its return value is lost.
///
/// # Examples
///
/// Creation from [`task::spawn`]:
///
/// ```
/// use nio::task;
///
/// # async fn doc() {
/// let join_handle: task::JoinHandle<_> = task::spawn(async {
///     // some work here
/// });
/// # }
/// ```
///
/// Creation from [`task::spawn_blocking`]:
///
/// ```
/// use nio::task;
///
/// # async fn doc() {
/// let join_handle: task::JoinHandle<_> = task::spawn_blocking(|| {
///     // some blocking work here
/// });
/// # }
/// ```
///
/// The generic parameter `T` in `JoinHandle<T>` is the return type of the spawned task.
/// If the return value is an `i32`, the join handle has type `JoinHandle<i32>`:
///
/// ```
/// use nio::task;
///
/// # async fn doc() {
/// let join_handle: task::JoinHandle<i32> = task::spawn(async {
///     5 + 3
/// });
/// # }
///
/// ```
///
/// If the task does not have a return value, the join handle has type `JoinHandle<()>`:
///
/// ```
/// use nio::task;
///
/// # async fn doc() {
/// let join_handle: task::JoinHandle<()> = task::spawn(async {
///     println!("I return nothing.");
/// });
/// # }
/// ```
///
/// Note that `handle.await` doesn't give you the return type directly. It is wrapped in a
/// `Result` because panics in the spawned task are caught by Nio. The `?` operator has
/// to be double chained to extract the returned value:
///
/// ```
/// use nio::task;
/// use std::io;
///
/// #[nio::main]
/// async fn main() -> io::Result<()> {
///     let join_handle: task::JoinHandle<Result<i32, io::Error>> = nio::spawn(async {
///         Ok(5 + 3)
///     });
///
///     let result = join_handle.await??;
///     assert_eq!(result, 8);
///     Ok(())
/// }
/// ```
///
/// If the task panics, the error is a [`JoinError`] that contains the panic:
///
/// ```
/// use nio::task;
/// use std::io;
/// use std::panic;
///
/// #[nio::main]
/// async fn main() -> io::Result<()> {
///     let join_handle: task::JoinHandle<Result<i32, io::Error>> = nio::spawn(async {
///         panic!("boom");
///     });
///
///     let err = join_handle.await.unwrap_err();
///     assert!(err.is_panic());
///     Ok(())
/// }
///
/// ```
/// Child being detached and outliving its parent:
///
/// ```no_run
/// use nio::task;
/// use nio::time;
/// use std::time::Duration;
///
/// # #[nio::main] async fn main() {
/// let original_task = task::spawn(async {
///     let _detached_task = task::spawn(async {
///         // Here we sleep to make sure that the first task returns before.
///         time::sleep(Duration::from_millis(10)).await;
///         // This will be called, even though the JoinHandle is dropped.
///         println!("♫ Still alive ♫");
///     });
/// });
///
/// original_task.await.expect("The task being joined has panicked");
/// println!("Original task is joined.");
///
/// // We make sure that the new task has time to run, before the main
/// // task returns.
///
/// time::sleep(Duration::from_millis(1000)).await;
/// # }
/// ```
///
/// [`task::spawn`]: crate::task::spawn()
/// [`task::spawn_blocking`]: crate::task::spawn_blocking
/// [`std::thread::JoinHandle`]: std::thread::JoinHandle
/// [`JoinError`]: crate::task::JoinError
pub struct JoinHandle<T> {
    raw: RawTask,
    _p: PhantomData<T>,
}
unsafe impl<T: Send> Send for JoinHandle<T> {}
unsafe impl<T: Send> Sync for JoinHandle<T> {}

impl<T> JoinHandle<T> {
    pub(super) fn new(raw: RawTask) -> JoinHandle<T> {
        JoinHandle {
            raw,
            _p: PhantomData,
        }
    }

    /// Abort the task associated with the handle.
    ///
    /// Awaiting a cancelled task might complete as usual if the task was
    /// already completed at the time it was cancelled, but most likely it
    /// will fail with a [cancelled] `JoinError`.
    ///
    /// Be aware that tasks spawned using [`spawn_blocking`] cannot be aborted
    /// because they are not async. If you call `abort` on a `spawn_blocking`
    /// task, then this *will not have any effect*, and the task will continue
    /// running normally. The exception is if the task has not started running
    /// yet; in that case, calling `abort` may prevent the task from starting.
    ///
    /// See also [the module level docs] for more information on cancellation.
    ///
    /// ```rust
    /// use nio::time;
    ///
    /// # #[nio::main]
    /// # async fn main() {
    /// let mut handles = Vec::new();
    ///
    /// handles.push(nio::spawn(async {
    ///    time::sleep(time::Duration::from_secs(10)).await;
    ///    true
    /// }));
    ///
    /// handles.push(nio::spawn(async {
    ///    time::sleep(time::Duration::from_secs(10)).await;
    ///    false
    /// }));
    ///
    /// for handle in &handles {
    ///     handle.abort();
    /// }
    ///
    /// for handle in handles {
    ///     assert!(handle.await.unwrap_err().is_cancelled());
    /// }
    /// # }
    /// ```
    ///
    /// [cancelled]: method@super::error::JoinError::is_cancelled
    /// [the module level docs]: crate::task#cancellation
    /// [`spawn_blocking`]: crate::task::spawn_blocking
    pub fn abort(&self) {
        unsafe {
            self.raw.abort_task();
        }
    }

    /// Checks if the task associated with this `JoinHandle` has finished.
    ///
    /// Please note that this method can return `false` even if [`abort`] has been
    /// called on the task. This is because the cancellation process may take
    /// some time, and this method does not return `true` until it has
    /// completed.
    ///
    /// ```rust
    /// use nio::time;
    ///
    /// # #[nio::main]
    /// # async fn main() {
    /// let handle1 = nio::spawn(async {
    ///     // do some stuff here
    /// });
    /// let handle2 = nio::spawn(async {
    ///     // do some other stuff here
    ///     time::sleep(time::Duration::from_secs(10)).await;
    /// });
    /// // Wait for the task to finish
    /// handle2.abort();
    /// time::sleep(time::Duration::from_secs(1)).await;
    /// assert!(handle1.is_finished());
    /// assert!(handle2.is_finished());
    /// # }
    /// ```
    /// [`abort`]: method@JoinHandle::abort
    pub fn is_finished(&self) -> bool {
        let state = self.raw.header().state.load();
        state.is(COMPLETE)
    }

    /// Returns a new `AbortHandle` that can be used to remotely abort this task.
    ///
    /// Awaiting a task cancelled by the `AbortHandle` might complete as usual if the task was
    /// already completed at the time it was cancelled, but most likely it
    /// will fail with a [cancelled] `JoinError`.
    ///
    /// ```rust
    /// use nio::{time, task};
    ///
    /// # #[nio::main]
    /// # async fn main() {
    /// let mut handles = Vec::new();
    ///
    /// handles.push(nio::spawn(async {
    ///    time::sleep(time::Duration::from_secs(10)).await;
    ///    true
    /// }));
    ///
    /// handles.push(nio::spawn(async {
    ///    time::sleep(time::Duration::from_secs(10)).await;
    ///    false
    /// }));
    ///
    /// let abort_handles: Vec<task::AbortHandle> = handles.iter().map(|h| h.abort_handle()).collect();
    ///
    /// for handle in abort_handles {
    ///     handle.abort();
    /// }
    ///
    /// for handle in handles {
    ///     assert!(handle.await.unwrap_err().is_cancelled());
    /// }
    /// # }
    /// ```
    /// [cancelled]: method@super::error::JoinError::is_cancelled
    #[must_use = "abort handles do nothing unless `.abort` is called"]
    pub fn abort_handle(&self) -> AbortHandle {
        AbortHandle::new(self.raw.clone())
    }

    /// Returns a [task ID] that uniquely identifies this task relative to other
    /// currently spawned tasks.
    ///
    /// **Note**: This is an [unstable API][unstable]. The public API of this type
    /// may break in 1.x releases. See [the documentation on unstable
    /// features][unstable] for details.
    ///
    /// [task ID]: crate::task::Id
    /// [unstable]: crate#unstable-features
    pub fn id(&self) -> Id {
        Id::new(&self.raw)
    }
}

impl<T> Unpin for JoinHandle<T> {}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut ret = Poll::Pending;

        // Try to read the task output. If the task is not yet complete, the
        // waker is stored and is notified once the task does complete.
        //
        // The function must go via the vtable, which requires erasing generic
        // types. To do this, the function "return" is placed on the stack
        // **before** calling the function and is passed into the function using
        // `*mut ()`.
        //
        // Safety:
        //
        // The type of `T` must match the task's output type.
        unsafe {
            self.raw
                .read_output(&mut ret as *mut _ as *mut (), cx.waker());
        }

        ret
    }
}

impl<T> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        // Try to unset `JOIN_INTEREST`. This must be done as a first step in
        // case the task concurrently completed.
        if self.raw.header().state.unset_join_interested().is_err() {
            // It is our responsibility to drop the output. This is critical as
            // the task output may not be `Send` and as such must remain with
            // the scheduler or `JoinHandle`. i.e. if the output remains in the
            // task structure until the task is deallocated, it may be dropped
            // by a Waker on any arbitrary thread.
            //
            // Panics are delivered to the user via the `JoinHandle`. Given that
            // they are dropping the `JoinHandle`, we assume they are not
            // interested in the panic and swallow it.
            let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                unsafe { self.raw.drop_task_or_output() };
            }));
        }
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
