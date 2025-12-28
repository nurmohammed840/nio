use mpmc_channel::MPMC;
use std::{collections::VecDeque, io, sync::Arc, thread, time::Duration};

type Channel<Task> = Arc<MPMC<VecDeque<Task>>>;

pub struct ThreadPool<Task: Runnable> {
    channel: Channel<Task>,

    // ----- config -----
    max_threads_limit: u16,
    timeout: Option<Duration>,
    stack_size: usize,
    name: Box<dyn Fn(usize) -> String + Send + Sync>,
}

impl<T: Runnable> Default for ThreadPool<T> {
    fn default() -> Self {
        Self {
            channel: Default::default(),

            timeout: Some(Duration::from_secs(10)),
            max_threads_limit: 512,
            stack_size: 0,
            name: Box::new(|id| format!("Worker: {id}")),
        }
    }
}

pub trait Runnable: Send + 'static {
    fn run(self);
}

impl<Task: Runnable> ThreadPool<Task> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn max_threads_limit(mut self, limit: u16) -> Self {
        assert!(limit > 0, "max threads limit must be greater than 0");
        self.max_threads_limit = limit;
        self
    }

    pub fn stack_size(mut self, stack_size: usize) -> Self {
        assert!(stack_size > 0, "stack size must be greater than 0");
        self.stack_size = stack_size;
        self
    }

    pub fn name<F>(mut self, f: F) -> Self
    where
        F: Fn(usize) -> String + 'static + Send + Sync,
    {
        self.name = Box::new(f);
        self
    }

    // ---------------  Getter  ------------------

    pub fn get_timeout(&self) -> Option<Duration> {
        self.timeout
    }

    pub fn get_stack_size(&self) -> Option<usize> {
        if self.stack_size == 0 {
            return None;
        }
        Some(self.stack_size)
    }

    pub fn get_max_threads_limit(&self) -> u16 {
        self.max_threads_limit
    }

    // -------------------------------------------

    pub fn thread_count(&self) -> usize {
        Arc::strong_count(&self.channel)
    }

    pub fn is_thread_limit_reached(&self) -> bool {
        self.thread_count() > self.get_max_threads_limit().into()
    }

    pub fn add_task_to_queue(&self, task: Task) {
        let mut tx = self.channel.produce();
        tx.push_back(task);
        tx.notify_one();
    }

    pub fn execute(&self, task: Task) {
        self.add_task_to_queue(task);

        if self.is_thread_limit_reached() {
            return;
        }

        let b = thread_builder(self);
        self.spawn(b)
            .expect("failed to spawn a thread in thread pool");
    }

    pub fn spawn(&self, thread_builder: thread::Builder) -> io::Result<thread::JoinHandle<()>> {
        let timeout = self.timeout;
        let channel = self.channel.clone();

        let worker = move || {
            let mut rx = channel.consume();
            loop {
                rx = match rx.pop_front() {
                    Some(task) => {
                        drop(rx);
                        task.run();
                        channel.consume()
                    }
                    None => match timeout {
                        None => rx.wait(),
                        Some(dur) => match rx.wait_timeout(dur) {
                            Ok(rx) => rx,
                            Err(_) => break,
                        },
                    },
                }
            }
        };
        thread_builder.spawn(worker)
    }
}

fn thread_builder<T: Runnable>(pool: &ThreadPool<T>) -> thread::Builder {
    let mut thread = thread::Builder::new();
    if let Some(size) = pool.get_stack_size() {
        thread = thread.stack_size(size);
    }
    let name = (pool.name)(pool.thread_count());
    if !name.is_empty() {
        thread = thread.name(name);
    }
    thread
}
