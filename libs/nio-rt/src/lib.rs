pub mod fs;
pub mod rt;

mod utils;

use std::time::Duration;

pub use rt::Runtime;
use rt::context::LocalContext;
pub use rt::context::RuntimeContext;
pub use rt::task::JoinHandle;

pub struct RuntimeBuilder {
    worker_threads: u8,
    worker_stack_size: Option<usize>,
    worker_name: Box<dyn Fn(u8) -> String>,

    event_interval: u32,

    max_blocking_threads: u16,
    thread_stack_size: usize,
    thread_name: Option<Box<dyn Fn(usize) -> String + Send + Sync>>,
    thread_timeout: Option<Duration>,
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self {
            worker_threads: std::thread::available_parallelism()
                .map(|nthread| nthread.get())
                .unwrap_or(1)
                .try_into()
                .unwrap(),

            event_interval: 61,

            worker_stack_size: None,
            thread_stack_size: 0,
            max_blocking_threads: 512,
            thread_timeout: Some(Duration::from_secs(10)),
            thread_name: Some(Box::new(|id| format!("Thread: {id}"))),
            worker_name: Box::new(|id| format!("Worker: {id}")),
        }
    }
}

impl RuntimeBuilder {
    pub fn new() -> RuntimeBuilder {
        Self::default()
    }

    pub fn worker_threads(mut self, val: u8) -> Self {
        assert!(val > 0);
        self.worker_threads = val;
        self
    }

    pub fn worker_stack_size(mut self, size: usize) -> Self {
        assert!(size > 0);
        self.worker_stack_size = Some(size);
        self
    }

    pub fn event_interval(mut self, tick: u32) -> Self {
        assert!(tick > 0);
        self.event_interval = tick;
        self
    }

    pub fn thread_stack_size(mut self, size: usize) -> Self {
        self.thread_stack_size = size;
        self
    }

    pub fn max_blocking_threads(mut self, val: u16) -> Self {
        self.max_blocking_threads = val;
        self
    }

    pub fn thread_timeout(mut self, dur: Option<Duration>) -> Self {
        self.thread_timeout = dur;
        self
    }

    pub fn thread_name<F>(mut self, f: F) -> Self
    where
        F: Fn(usize) -> String + 'static + Send + Sync,
    {
        self.thread_name = Some(Box::new(f));
        self
    }

    pub fn worker_name<F>(mut self, f: F) -> Self
    where
        F: Fn(u8) -> String + 'static,
    {
        self.worker_name = Box::new(f);
        self
    }
}

pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    RuntimeContext::with(|ctx| ctx.spawn_blocking(f))
}

pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    RuntimeContext::with(|ctx| ctx.spawn(future))
}

pub fn spawn_pinned<F, Fut>(future: F) -> JoinHandle<Fut::Output>
where
    F: FnOnce() -> Fut + Send,
    Fut: Future + 'static,
    Fut::Output: Send + 'static,
{
    RuntimeContext::with(|ctx| ctx.spawn_pinned(future))
}

pub fn spawn_pinned_at<F, Fut>(worker: u8, future: F) -> JoinHandle<Fut::Output>
where
    F: FnOnce() -> Fut + Send,
    Fut: Future + 'static,
    Fut::Output: Send + 'static,
{
    RuntimeContext::with(|ctx| ctx.spawn_pinned_at(worker, future))
}

pub fn spawn_local<Fut>(future: Fut) -> JoinHandle<Fut::Output>
where
    Fut: Future + 'static,
    Fut::Output: 'static,
{
    LocalContext::with(|ctx| ctx.spawn_local(future))
}
