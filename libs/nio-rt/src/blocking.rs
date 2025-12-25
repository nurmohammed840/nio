use nio_threadpool::Runnable;

pub struct BlockingTask {
    pub task: nio_task::BlockingTask
}

impl Runnable for BlockingTask {
    fn run(self) {
        self.task.run();
    }
}