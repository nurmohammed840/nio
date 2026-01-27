#[doc = include_str!("../README.md")]

pub mod fs;
pub mod net;

mod driver;
mod local_waker;
mod rt;
mod timer;
mod utils;

use std::num::NonZero;
use std::{num::NonZeroUsize, time::Duration};

pub use nio_macros::*;
pub use nio_task::id as task_id;
pub use nio_task::{AbortHandle, JoinError, JoinHandle, TaskId};
pub use rt::{
    Runtime,
    context::{LocalContext, RuntimeContext},
    metrics,
    WorkerId
};
pub use timer::{
    interval::{Interval, interval},
    sleep::{Sleep, sleep},
    timeout::{Timeout, timeout},
};

use crate::rt::context::{no_rt_found_panic, NioContext};

pub struct RuntimeBuilder {
    worker_threads: u8,
    worker_stack_size: Option<NonZeroUsize>,
    worker_name: Box<dyn Fn(u8) -> String + Send + Sync>,

    event_interval: u32,
    min_tasks_per_worker: Option<NonZero<u64>>,

    threadpool_load_factor: usize,
    max_blocking_threads: u16,
    thread_stack_size: usize,
    thread_timeout: Option<Duration>,
    thread_name: Option<Box<dyn Fn(usize) -> String + Send + Sync>>,

    #[cfg(feature = "metrics")]
    measurement: Option<Box<dyn metrics::Measurement>>,
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self {
            worker_threads: std::thread::available_parallelism()
                .map(|nthread| nthread.get())
                .unwrap_or(1)
                .try_into()
                .unwrap(),

            worker_stack_size: None,
            worker_name: Box::new(|id| format!("Worker: {id}")),

            event_interval: 61,
            min_tasks_per_worker: None,

            threadpool_load_factor: 2,
            max_blocking_threads: 512,
            thread_stack_size: 0,
            thread_timeout: Some(Duration::from_secs(10)),
            thread_name: Some(Box::new(|id| format!("Thread: {id}"))),

            #[cfg(feature = "metrics")]
            measurement: Some(Box::new(metrics::NoMeasurement)),
        }
    }
}

impl RuntimeBuilder {
    pub fn new() -> RuntimeBuilder {
        Self::default()
    }

    #[allow(warnings)]
    pub fn measurement(mut self, metrics: impl metrics::Measurement + 'static) -> Self {
        #[cfg(feature = "metrics")]
        {
            self.measurement = Some(Box::new(metrics));
        }
        self
    }

    pub fn worker_threads(mut self, val: u8) -> Self {
        assert!(val > 0);
        self.worker_threads = val;
        self
    }

    pub fn worker_stack_size(mut self, size: usize) -> Self {
        self.worker_stack_size = NonZeroUsize::new(size);
        self
    }

    pub fn event_interval(mut self, tick: u32) -> Self {
        assert!(tick > 0);
        self.event_interval = tick;
        self
    }

    pub fn min_tasks_per_worker(mut self, count: usize) -> Self {
        assert_ne!(count, 0);
        self.min_tasks_per_worker = NonZero::new(count.try_into().unwrap());
        self
    }

    pub fn thread_stack_size(mut self, size: usize) -> Self {
        self.thread_stack_size = size;
        self
    }

    pub fn threadpool_load_factor(mut self, factor: usize) -> Self {
        self.threadpool_load_factor = factor;
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
        F: Fn(u8) -> String + 'static + Send + Sync,
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
    NioContext::get(|ctx| match ctx {
        NioContext::None => no_rt_found_panic(),
        NioContext::Runtime(ctx) => ctx.spawn(future),
        NioContext::Local(ctx) => ctx.spawn(future),
    })
}

pub fn spawn_pinned<F, Fut>(future: F) -> JoinHandle<Fut::Output>
where
    F: FnOnce() -> Fut + Send,
    Fut: Future + 'static,
    Fut::Output: Send + 'static,
{
    NioContext::get(|ctx| match ctx {
        NioContext::None => no_rt_found_panic(),
        NioContext::Runtime(ctx) => ctx.spawn_pinned(future),
        NioContext::Local(ctx) => ctx.spawn_pinned(future),
    })
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
