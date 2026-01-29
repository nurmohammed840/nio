pub mod context;
mod event_loop;
pub mod metrics;
pub mod task;
mod task_queue;
mod worker;

use crate::{LocalContext, RuntimeBuilder, driver, rt::event_loop::EventLoop};
use std::{io, rc::Rc, sync::Arc, thread};

use nio_threadpool::ThreadPool;

use context::RuntimeContext;
use worker::Workers;

pub use worker::WorkerId;

const LOCAL_QUEUE_CAP: usize = 512;

impl RuntimeBuilder {
    pub fn rt(mut self) -> io::Result<Runtime> {
        let min_tasks_per_worker = match self.min_tasks_per_worker {
            Some(count) => count.get(),
            None => (self.worker_threads as u64 / 2).max(1),
        };

        let (workers, drivers) = Workers::new(self.worker_threads, min_tasks_per_worker)?;
        let context = Arc::new(RuntimeContext {
            workers,
            #[cfg(feature = "metrics")]
            measurement: {
                let mut metrics = self.measurement.take().unwrap();
                metrics.init(self.worker_threads.into());
                metrics
            },
            threadpool: ThreadPool::new()
                .max_threads_limit(self.max_blocking_threads)
                .load_factor(self.threadpool_load_factor)
                .stack_size(self.thread_stack_size)
                .timeout(self.thread_timeout)
                .name(self.thread_name.take().unwrap()),
        });

        let tick = self.event_interval;

        for (id, driver) in drivers.into_iter().enumerate() {
            let id = id as u8;
            let runtime_ctx = context.clone();

            self.create_thread(id)
                .spawn(move || {
                    EventLoop::new(id, driver, runtime_ctx, tick, LOCAL_QUEUE_CAP).run();
                })
                .unwrap_or_else(|err| panic!("failed to spawn worker thread {id}; {err}"));
        }

        Ok(Runtime { context })
    }

    pub fn build(mut self) -> io::Result<LocalRuntime> {
        let min_tasks_per_worker = match self.min_tasks_per_worker {
            Some(count) => count.get(),
            None => (self.worker_threads as u64 / 2).max(1),
        };

        let (workers, drivers) = Workers::new(self.worker_threads, min_tasks_per_worker)?;
        let runtime_ctx = Arc::new(RuntimeContext {
            workers,
            #[cfg(feature = "metrics")]
            measurement: {
                let mut metrics = self.measurement.take().unwrap();
                metrics.init(self.worker_threads.into());
                metrics
            },
            threadpool: ThreadPool::new()
                .max_threads_limit(self.max_blocking_threads)
                .load_factor(self.threadpool_load_factor)
                .stack_size(self.thread_stack_size)
                .timeout(self.thread_timeout)
                .name(self.thread_name.take().unwrap()),
        });

        let tick = self.event_interval;
        let mut drivers = drivers.into_iter().enumerate();

        let (id, driver) = drivers.next().unwrap();
        let main_event_loop =
            EventLoop::new(id as u8, driver, runtime_ctx.clone(), tick, LOCAL_QUEUE_CAP);

        for (id, driver) in drivers {
            let id = id as u8;
            let runtime_ctx = runtime_ctx.clone();

            self.create_thread(id)
                .spawn(move || {
                    EventLoop::new(id, driver, runtime_ctx, tick, LOCAL_QUEUE_CAP).run();
                })
                .unwrap_or_else(|err| panic!("failed to spawn worker thread {id}; {err}"));
        }

        Ok(LocalRuntime { main_event_loop })
    }
}

pub struct LocalRuntime {
    main_event_loop: EventLoop,
}

impl LocalRuntime {
    pub fn local_context(&self) -> Rc<LocalContext> {
        self.main_event_loop.local_ctx.clone()
    }

    pub fn runtime_context(&self) -> Arc<RuntimeContext> {
        self.main_event_loop.local_ctx.runtime_ctx.clone()
    }

    pub fn block_on<Fut: Future>(&mut self, fut: Fut) -> Fut::Output {
        self.main_event_loop.run_until(fut)
    }
}

impl std::ops::Deref for LocalRuntime {
    type Target = LocalContext;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.main_event_loop.local_ctx
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
        Fut: Future,
        Fut::Output: Send,
    {
        let id = self.context.workers.id(0);
        let (task, join) = unsafe { task::LocalScheduler::spawn(id, self.context.clone(), fut()) };
        self.context.send_task_at(id, task);
        nio_future::block_on(join).unwrap()
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
