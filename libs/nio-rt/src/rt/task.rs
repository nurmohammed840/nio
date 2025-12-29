use std::sync::Arc;

pub use nio_task::{JoinHandle, Status};
use nio_threadpool::Runnable;

use super::{
    context::{Context, RuntimeContext},
    worker::WorkerId,
};

pub type Task = nio_task::Task<()>;

pub struct Scheduler {
    pub runtime_ctx: Arc<RuntimeContext>,
}

pub struct LocalScheduler {
    pub pinned: WorkerId,
    pub runtime_ctx: Arc<RuntimeContext>,
}

impl nio_task::Scheduler for Scheduler {
    fn schedule(&self, task: Task) {
        self.runtime_ctx.send_task_to_least_loaded_worker(task);
    }
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

pub struct BlockingTask {
    pub task: nio_task::BlockingTask,
}

impl BlockingTask {
    pub fn new<F, R>(f: F) -> (BlockingTask, JoinHandle<R>)
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (task, join) = nio_task::BlockingTask::new(f);
        (Self { task }, join)
    }
}

impl Runnable for BlockingTask {
    fn run(self) {
        self.task.run();
    }
}
