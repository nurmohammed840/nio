use crate::{
    blocking::SpawnBlocking,
    runtime::{
        self,
        task::{BlockingTask, JoinHandle},
    },
};

/// Runs the provided closure on a thread where blocking is acceptable.
///
/// In general, issuing a blocking call or performing a lot of compute in a
/// future without yielding is problematic, as it may prevent the executor from
/// driving other futures forward. This function runs the provided closure on a
/// thread dedicated to blocking operations. See the [CPU-bound tasks and
/// blocking code][blocking] section for more information.
///
/// Nio will spawn more blocking threads when they are requested through this
/// function until the upper limit configured on the [`Builder`] is reached.
/// After reaching the upper limit, the tasks are put in a queue.
/// The thread limit is very large by default, because `spawn_blocking` is often
/// used for various kinds of IO operations that cannot be performed
/// asynchronously.  When you run CPU-bound code using `spawn_blocking`, you
/// should keep this large upper limit in mind. When running many CPU-bound
/// computations, a semaphore or some other synchronization primitive should be
/// used to limit the number of computation executed in parallel. Specialized
/// CPU-bound executors, such as [rayon], may also be a good fit.
///
/// This function is intended for non-async operations that eventually finish on
/// their own. If you want to spawn an ordinary thread, you should use
/// [`thread::spawn`] instead.
///
/// Be aware that tasks spawned using `spawn_blocking` cannot be aborted
/// because they are not async. If you call [`abort`] on a `spawn_blocking`
/// task, then this *will not have any effect*, and the task will continue
/// running normally. The exception is if the task has not started running
/// yet; in that case, calling `abort` may prevent the task from starting.
///
/// When you shut down the executor, it will wait indefinitely for all blocking operations to
/// finish. You can use [`shutdown_timeout`] to stop waiting for them after a
/// certain timeout. Be aware that this will still not cancel the tasks — they
/// are simply allowed to keep running after the method returns.  It is possible
/// for a blocking task to be cancelled if it has not yet started running, but this
/// is not guaranteed.
///
/// Note that if you are using the single threaded runtime, this function will
/// still spawn additional threads for blocking operations. The current-thread
/// scheduler's single thread is only used for asynchronous code.
///
/// # Related APIs and patterns for bridging asynchronous and blocking code
///
/// In simple cases, it is sufficient to have the closure accept input
/// parameters at creation time and return a single value (or struct/tuple, etc.).
///
/// For more complex situations in which it is desirable to stream data to or from
/// the synchronous context, the [`mpsc channel`] has `blocking_send` and
/// `blocking_recv` methods for use in non-async code such as the thread created
/// by `spawn_blocking`.
///
/// Another option is [`SyncIoBridge`] for cases where the synchronous context
/// is operating on byte streams.  For example, you might use an asynchronous
/// HTTP client such as [hyper] to fetch data, but perform complex parsing
/// of the payload body using a library written for synchronous I/O.
///
/// Finally, see also [Bridging with sync code][bridgesync] for discussions
/// around the opposite case of using Nio as part of a larger synchronous
/// codebase.
///
/// [`Builder`]: struct@crate::runtime::Builder
/// [blocking]: ../index.html#cpu-bound-tasks-and-blocking-code
/// [rayon]: https://docs.rs/rayon
/// [`SyncIoBridge`]: https://docs.rs/tokio-util/latest/tokio_util/io/struct.SyncIoBridge.html
/// [hyper]: https://docs.rs/hyper
/// [`thread::spawn`]: fn@std::thread::spawn
/// [`shutdown_timeout`]: fn@crate::runtime::Runtime::shutdown_timeout
/// [`AtomicBool`]: struct@std::sync::atomic::AtomicBool
/// [`abort`]: crate::task::JoinHandle::abort
///
/// # Examples
///
/// Pass an input value and receive result of computation:
///
/// ```
/// use nio::task;
///
/// # async fn docs() -> Result<(), Box<dyn std::error::Error>>{
/// // Initial input
/// let mut v = "Hello, ".to_string();
/// let res = task::spawn_blocking(move || {
///     // Stand-in for compute-heavy work or using synchronous APIs
///     v.push_str("world");
///     // Pass ownership of the value back to the asynchronous context
///     v
/// }).await?;
///
/// // `res` is the value returned from the thread
/// assert_eq!(res.as_str(), "Hello, world");
/// # Ok(())
/// # }
/// ```
///
/// Use a channel:
///
/// ```
/// use nio::task;
/// use tokio::sync::mpsc;
///
/// # async fn docs() {
/// let (tx, mut rx) = mpsc::channel(2);
/// let start = 5;
/// let worker = task::spawn_blocking(move || {
///     for x in 0..10 {
///         // Stand in for complex computation
///         tx.blocking_send(start + x).unwrap();
///     }
/// });
///
/// let mut acc = 0;
/// while let Some(v) = rx.recv().await {
///     acc += v;
/// }
/// assert_eq!(acc, 95);
/// worker.await.unwrap();
/// # }
/// ```
pub fn spawn_blocking<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let (task, join_handle) = BlockingTask::new(f);
    runtime::context::with(|ctx| ctx.thread_pool.spawn_blocking(task));
    join_handle
}
