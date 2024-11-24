use super::{context, task::JoinHandle, Builder, EnterGuard, Handle};
use crate::{blocking::ThreadPool, io::reactor::Reactor, scheduler::Scheduler};
use std::{future::Future, sync::Arc, thread};

pub struct Runtime {
    handle: Handle,
    _workers: Vec<thread::JoinHandle<()>>,
    _reactor: thread::JoinHandle<()>,
}

impl Runtime {
    pub fn new(config: Builder, thread_pool: ThreadPool) -> Self {
        let (scheduler, task_queues) = Scheduler::new(config.worker_threads);

        let (reactor, reactor_ctx) = Reactor::new(1024).unwrap();

        let _reactor = thread::Builder::new()
            .name("Reactor".into())
            .spawn(|| reactor.run())
            .expect("failed to spawn reactor thread");

        let context = Arc::new(context::RuntimeContext {
            scheduler,
            thread_pool,
            reactor_ctx: Some(reactor_ctx),
        });

        let workers = task_queues
            .into_iter()
            .enumerate()
            .map(|(i, mut queue)| {
                let context = context.clone();

                let mut thread = thread::Builder::new().name(format!("Worker({})", i + 1));
                if let Some(size) = config.stack_size {
                    thread = thread.stack_size(size);
                }

                let worker = move || {
                    context.init();

                    queue.registered();
                    while let Some(mut task) = queue.fetch() {
                        if task.process() {
                            queue.add(task);
                        }
                    }
                    queue.deregister();
                };
                thread.spawn(worker).expect("failed to spawn worker thread")
            })
            .collect::<Vec<_>>();

        Self {
            _workers: workers,
            _reactor,
            handle: Handle { ctx: context },
        }
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.handle.spawn(future)
    }

    pub fn handle(&self) -> &Handle {
        &self.handle
    }

    pub fn enter(&self) -> EnterGuard {
        self.handle.ctx.clone().init();
        EnterGuard::new()
    }

    pub fn block_on<Fut>(&self, fut: Fut) -> Fut::Output
    where
        Fut: Future,
    {
        self.handle.ctx.clone().init();
        crate::future::block_on(fut)
    }
}
