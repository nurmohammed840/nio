#![allow(unused)]
pub mod context;
pub mod task;
mod task_counter;
mod worker;

use crate::{RuntimeBuilder, rt::worker::WorkerId};
use std::{sync::Arc, thread};

use nio_threadpool::ThreadPool;

use context::{LocalContext, RuntimeContext};
use worker::Workers;

impl RuntimeBuilder {
    pub fn rt(mut self) -> Runtime {
        let context = Arc::new(RuntimeContext {
            workers: Workers::new(self.worker_threads),
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
    config: RuntimeBuilder,
    context: Arc<RuntimeContext>,
}

impl Runtime {
    fn create_thread(&self, id: u8) -> thread::Builder {
        let mut thread = thread::Builder::new();
        if let Some(size) = self.config.worker_stack_size {
            thread = thread.stack_size(size.get());
        }
        let name = (self.config.worker_name)(id);
        if !name.is_empty() {
            thread = thread.name(name);
        }
        thread
    }

    pub fn block_on<Fut>(self, fut: Fut) -> Fut::Output
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        let event_interval = self.config.event_interval;
        for id in 0..self.config.worker_threads {
            let runtime_ctx = self.context.clone();
            self.create_thread(id)
                .spawn(move || {
                    Workers::job(
                        LocalContext::new(runtime_ctx.workers.id(id), 512, runtime_ctx),
                        event_interval,
                    )
                })
                .unwrap_or_else(|err| panic!("failed to spawn worker thread {id}; {err}"));
        }

        drop(self.config);
        nio_future::block_on(self.context.spawn_pinned_at(0, || fut)).unwrap()
    }
}
