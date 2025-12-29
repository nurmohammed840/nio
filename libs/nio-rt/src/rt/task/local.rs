use nio_task::{JoinHandle, Task};
use std::{mem::ManuallyDrop, pin::Pin, sync::Arc, task::Poll};

use crate::rt::{
    context::{Context, RuntimeContext},
    worker::WorkerId,
};

pub struct LocalScheduler {
    pinned: WorkerId,
    runtime_ctx: Arc<RuntimeContext>,
}

impl nio_task::Scheduler for LocalScheduler {
    fn schedule(&self, task: Task) {
        Context::get(|ctx| match ctx {
            Context::Local(ctx) if self.pinned == ctx.worker_id => {
                ctx.add_task_to_local_queue(task)
            }
            _ => self.runtime_ctx.send_task_at(self.pinned, task),
        });
    }
}

impl LocalScheduler {
    pub fn spawn<F>(
        pinned: WorkerId,
        runtime_ctx: Arc<RuntimeContext>,
        future: F,
    ) -> (Task, JoinHandle<F::Output>)
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        let future = LocalFuture {
            worker_id: pinned,
            fut: ManuallyDrop::new(future),
        };
        unsafe {
            Task::new_unchecked(
                (),
                future,
                LocalScheduler {
                    pinned,
                    runtime_ctx,
                },
            )
        }
    }
}

struct LocalFuture<F> {
    worker_id: WorkerId,
    fut: ManuallyDrop<F>,
}

fn is_same_worker(f: impl FnOnce(WorkerId) -> bool) -> bool {
    Context::get(|ctx| match ctx {
        Context::Local(ctx) => f(ctx.worker_id),
        Context::None | Context::Global(_) => false,
    })
}

const ABORT: u8 = 1;
const PANIC: u8 = 2;

#[cfg(debug_assertions)]
const DROP_STRATEGY: u8 = ABORT;

#[cfg(not(debug_assertions))]
const DROP_STRATEGY: u8 = PANIC;

impl<F> Drop for LocalFuture<F> {
    fn drop(&mut self) {
        if DROP_STRATEGY == ABORT {
            if !is_same_worker(|id| self.worker_id == id) {
                eprintln!("local task dropped by a thread that didn't spawn it");
                std::process::abort();
            }
        }
        if DROP_STRATEGY == PANIC {
            assert!(
                is_same_worker(|id| self.worker_id == id),
                "local task dropped by a thread that didn't spawn it"
            );
        }
        unsafe {
            ManuallyDrop::drop(&mut self.fut);
        }
    }
}

impl<F: Future> Future for LocalFuture<F> {
    type Output = F::Output;
    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        debug_assert!(
            is_same_worker(|id| self.worker_id == id),
            "local task polled by a thread that didn't spawn it"
        );
        unsafe { self.map_unchecked_mut(|c| &mut *c.fut).poll(cx) }
    }
}
