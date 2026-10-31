use nio_task::JoinHandle;
use nio_threadpool::Runnable;

pub struct BlockingTask {
    task: nio_task::BlockingTask,
}

impl Runnable for BlockingTask {
    fn run(self) {
        self.task.run();
    }
}

impl BlockingTask {
    pub fn spawn<F, R>(f: F) -> (BlockingTask, JoinHandle<R>)
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (task, join) = nio_task::BlockingTask::new(f);
        (Self { task }, join)
    }
}
