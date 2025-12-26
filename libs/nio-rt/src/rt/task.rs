pub use nio_task::{JoinHandle, Status};
use nio_threadpool::Runnable;

use crate::rt::local_context::LocalContext;

pub enum TaskKind {
    Sendable,
    Pinned(u8),
}

pub struct Metadata {
    pub kind: TaskKind,
}

pub type Task = nio_task::Task<Metadata>;

pub struct Scheduler;

impl nio_task::Scheduler<Metadata> for Scheduler {
    fn schedule(&self, task: Task) {
        LocalContext::with(move |context| match task.metadata().kind {
            TaskKind::Sendable => {}
            TaskKind::Pinned(id) if context.worker_id == id => {
                context.add_task_to_local_queue(task);
            }
            TaskKind::Pinned(id) => {
                let workers = &context.runtime_ctx.workers;
                workers.shared_queues[id as usize].push(task);
                workers.task_counters[id as usize].increase_shared();
            }
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
