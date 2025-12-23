use crate::blocking::BlockingTask;
use nio_threadpool::ThreadPool;

use crate::RuntimeConfig;

impl RuntimeConfig {
    pub fn rt(self) -> Runtime {
        let threadpool = ThreadPool::new()
            .max_threads_limit(self.max_blocking_threads)
            .stack_size(self.thread_stack_size)
            .timeout(self.thread_timeout)
            .name(self.thread_name);

        Runtime {
            _workers: Box::new([]),
            _threadpool: threadpool,
        }
    }
}

pub struct Runtime {
    _workers: Box<[Worker]>,
    _threadpool: ThreadPool<BlockingTask>,
}

// -------------------------------------------

pub struct Worker {}
