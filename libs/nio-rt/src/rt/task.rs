pub use nio_task::{JoinHandle, Status};
use nio_threadpool::Runnable;

pub enum TaskKind {
    Sendable,
    Pinned(u8),
}

pub struct Metadata {
    pub kind: TaskKind,
}

pub type Task = nio_task::Task<Metadata>;

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
