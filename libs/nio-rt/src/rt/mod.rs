pub mod context;
pub mod metrics;
pub mod task;
mod task_queue;
mod worker;

use crate::{RuntimeBuilder, driver};
use std::{io, sync::Arc, thread};

use nio_threadpool::ThreadPool;

use context::{LocalContext, RuntimeContext};
use worker::Workers;

impl RuntimeBuilder {
    pub fn rt(mut self) -> io::Result<Runtime> {
        let (workers, drivers) = Workers::new(self.worker_threads)?;
        let context = Arc::new(RuntimeContext {
            workers,
            threadpool: ThreadPool::new()
                .max_threads_limit(self.max_blocking_threads)
                .stack_size(self.thread_stack_size)
                .timeout(self.thread_timeout)
                .name(self.thread_name.take().unwrap()),
        });

        let local_queue_cap: usize = 512;
        let event_interval = self.event_interval;

        for (id, driver) in drivers.into_iter().enumerate() {
            let id = id as u8;
            let context = context.clone();
            let worker_id = context.workers.id(id);

            self.create_thread(id)
                .spawn(move || {
                    let io_registry = driver.registry_owned().unwrap();
                    Workers::job(
                        LocalContext::new(worker_id, local_queue_cap, context, io_registry),
                        event_interval,
                        driver,
                    )
                })
                .unwrap_or_else(|err| panic!("failed to spawn worker thread {id}; {err}"));
        }

        Ok(Runtime { context })
    }
}

pub struct Runtime {
    context: Arc<RuntimeContext>,
}

impl Runtime {
    pub fn context(&self) -> Arc<RuntimeContext> {
        self.context.clone()
    }

    pub fn block_on<F, Fut>(&self, fut: F) -> Fut::Output
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future + 'static,
        Fut::Output: Send + 'static,
    {
        nio_future::block_on(self.context.spawn_pinned_at(0, fut)).unwrap()
    }
}

impl std::ops::Deref for Runtime {
    type Target = Arc<RuntimeContext>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl RuntimeBuilder {
    fn create_thread(&self, id: u8) -> thread::Builder {
        let mut thread = thread::Builder::new();
        if let Some(size) = self.worker_stack_size {
            thread = thread.stack_size(size.get());
        }
        let name = (self.worker_name)(id);
        if !name.is_empty() {
            thread = thread.name(name);
        }
        thread
    }
}
