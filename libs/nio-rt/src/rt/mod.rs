#![allow(unused)]
mod context;
mod local_context;
mod task_counter;
mod worker;

use crate::RuntimeConfig;
use std::{sync::Arc, thread};

use nio_threadpool::ThreadPool;

use context::RuntimeContext;
use worker::Worker;

impl RuntimeConfig {
    pub fn rt(mut self) -> Runtime {
        let context = Arc::new(RuntimeContext {
            workers: (0..self.worker_threads).map(|_| Worker::new()).collect(),
            threadpool: ThreadPool::new()
                .max_threads_limit(self.max_blocking_threads)
                .stack_size(self.thread_stack_size)
                .timeout(self.thread_timeout)
                .name(self.thread_name.take().unwrap()),
        });
        Runtime {
            context,
            config: self,
        }
    }
}

pub struct Runtime {
    config: RuntimeConfig,
    context: Arc<RuntimeContext>,
}

impl Runtime {
    fn create_thread(&self, id: u8) -> thread::Builder {
        let mut thread = thread::Builder::new();
        if let Some(size) = self.config.worker_stack_size {
            thread = thread.stack_size(size);
        }
        let name = (self.config.worker_name)(id);
        if !name.is_empty() {
            thread = thread.name(name);
        }
        thread
    }

    pub fn block_on(self) {
        let event_interval = self.config.event_interval;
        for id in 1..self.config.worker_threads {
            let runtime = self.context.clone();
            self.create_thread(id)
                .spawn(move || Worker::job(id, event_interval, runtime))
                .expect(&format!("failed to spawn worker thread: {id}"));
        }
        drop(self.config);

        Worker::job(0, event_interval, self.context)
    }
}
