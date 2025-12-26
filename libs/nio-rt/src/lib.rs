pub mod rt;

use std::time::Duration;

pub struct RuntimeConfig {
    worker_threads: u8,
    worker_stack_size: Option<usize>,
    worker_name: Box<dyn Fn(u8) -> String>,

    event_interval: u32,

    max_blocking_threads: u16,
    thread_stack_size: usize,
    thread_name: Option<Box<dyn Fn(usize) -> String + Send + Sync>>,
    thread_timeout: Option<Duration>,
}

impl Default for RuntimeConfig {
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

impl RuntimeConfig {
    pub fn new() -> RuntimeConfig {
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
