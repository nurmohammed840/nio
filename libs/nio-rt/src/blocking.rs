use nio_threadpool::Runnable;

pub struct BlockingTask;
impl Runnable for BlockingTask {
    fn run(self) {
        todo!()
    }
}