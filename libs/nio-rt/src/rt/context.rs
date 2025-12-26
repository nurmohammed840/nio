use super::*;

use nio_task::{JoinHandle, Task};

pub(crate) struct Blocking {
    pub task: nio_task::BlockingTask,
}

impl nio_threadpool::Runnable for Blocking {
    fn run(self) {
        self.task.run();
    }
}

pub struct RuntimeContext {
    pub(crate) workers: Box<[Worker]>,
    pub(crate) threadpool: ThreadPool<Blocking>,
}

impl RuntimeContext {
    pub fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (task, join) = nio_task::BlockingTask::new(f);
        self.threadpool.execute(Blocking { task });
        join
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (task, join) = nio_task::Task::new(future, |_| {});
        self.workers[0].shared_queue.push(task);
        join
    }
}
