use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

mod blocking;
pub use blocking::spawn_blocking;

pub use crate::runtime::task::{AbortHandle, Id, JoinError, JoinHandle};
use crate::runtime::{self, context};

#[cfg_attr(docsrs, doc(cfg(feature = "rt")))]
pub async fn yield_now() {
    /// Yield implementation
    struct YieldNow {
        yielded: bool,
    }
    impl Future for YieldNow {
        type Output = ();
        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            if self.yielded {
                return Poll::Ready(());
            }
            self.yielded = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
    YieldNow { yielded: false }.await;
}

pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    if std::mem::size_of::<F>() > crate::BOX_FUTURE_THRESHOLD {
        spawn_inner(Box::pin(future))
    } else {
        spawn_inner(future)
    }
}

fn spawn_inner<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    context::with(|ctx| spawn_task(future, ctx))
}

pub(crate) fn spawn_task<F>(future: F, ctx: &Arc<context::RuntimeContext>) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let (task, handler) = runtime::Task::new(future, ctx.scheduler.clone());
    ctx.scheduler.spawn(task);
    handler
}
