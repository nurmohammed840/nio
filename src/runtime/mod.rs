pub(crate) mod context;
mod handle;
pub(crate) mod multithread;
pub(crate) mod task;

pub use handle::{EnterGuard, Handle};
use std::{thread, time::Duration};
pub(crate) use task::Task;

pub use multithread::Runtime;

use crate::blocking::ThreadPoolConfig;

#[derive(Debug, Clone)]
pub struct Builder {
    pub worker_threads: usize,
    pub max_blocking_threads: u16,
    pub stack_size: Option<usize>,
    pub thread_name: Option<String>,
    pub thread_timeout: Option<Duration>,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            worker_threads: thread::available_parallelism()
                .map(|nthread| nthread.get())
                .unwrap_or(4),

            stack_size: None,
            max_blocking_threads: 512,
            thread_timeout: Some(Duration::from_secs(10)),
            thread_name: None,
        }
    }
}

impl Builder {
    pub fn new_multi_thread() -> Self {
        Self::default()
    }

    pub fn worker_threads(&mut self, val: usize) -> &mut Self {
        assert!(val > 0, "Worker threads cannot be set to 0");
        self.worker_threads = val;
        self
    }

    pub fn enable_all(&mut self) -> &mut Self {
        self
    }

    pub fn stack_size(&mut self, stack_size: usize) -> &mut Self {
        self.stack_size = Some(stack_size);
        self
    }

    pub fn max_blocking_threads(&mut self, val: u16) -> &mut Self {
        assert!(val > 0, "Max blocking threads cannot be set to 0");
        self.max_blocking_threads = val;
        self
    }

    pub fn thread_timeout(&mut self, dur: Option<Duration>) -> &mut Self {
        self.thread_timeout = dur;
        self
    }

    pub fn thread_name(&mut self, name: impl Into<String>) -> &mut Self {
        self.thread_name = Some(name.into());
        self
    }

    pub fn build(&self) -> Result<multithread::Runtime, String> {
        let thread_pool = ThreadPoolConfig {
            max_workers: self.max_blocking_threads,
            timeout: self.thread_timeout,
            stack_size: self.stack_size,
            name: self.thread_name.clone(),
        };
        Ok(multithread::Runtime::new(self.clone(), thread_pool.into()))
    }
}
