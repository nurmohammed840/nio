use mpmc_channel::MPMC;
use std::{collections::VecDeque, io, num::NonZero, sync::Arc, thread, time::Duration};

type Channel<Task> = Arc<MPMC<Queue<Task>>>;

struct Queue<Task> {
    tasks: VecDeque<Task>,
    count: usize,
}

pub struct ThreadPool<Task: Runnable> {
    channel: Channel<Task>,

    // ----- config -----
    max_threads_limit: u16,
    timeout: Option<Duration>,
    stack_size: Option<NonZero<usize>>,
    load_factor: usize,
    name: Box<dyn Fn(usize) -> String + Send + Sync>,
}

impl<T: Runnable> Default for ThreadPool<T> {
    fn default() -> Self {
        Self {
            channel: Arc::new(MPMC::new(Queue {
                tasks: VecDeque::with_capacity(64),
                count: 0,
            })),

            timeout: Some(Duration::from_secs(7)),
            max_threads_limit: 32,
            load_factor: 2,
            stack_size: None,
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
        self.stack_size = NonZero::new(stack_size);
        self
    }

    pub fn load_factor(mut self, factor: usize) -> Self {
        assert!(factor != 0, "threadpool load factor must be > 0");
        self.load_factor = factor;
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
        self.stack_size.map(|size| size.get())
    }

    pub fn get_max_threads_limit(&self) -> u16 {
        self.max_threads_limit
    }

    // -------------------------------------------

    pub fn thread_count(&self) -> usize {
        Arc::strong_count(&self.channel) - 1
    }

    pub fn is_thread_limit_reached(&self) -> bool {
        self.thread_count() > self.get_max_threads_limit().into()
    }

    pub fn add_task_to_queue(&self, task: Task) -> usize {
        let mut tx = self.channel.produce();
        tx.tasks.push_back(task);
        tx.count += 1;
        let task_count = tx.count;
        tx.notify_one();
        task_count
    }

    pub fn execute(&self, task: Task) {
        let task_count = self.add_task_to_queue(task);

        let thread_count = self.thread_count();
        if thread_count > self.get_max_threads_limit().into() {
            return;
        }

        let threshold = thread_count * self.load_factor;
        if task_count <= threshold {
            return;
        }

        let b = self.thread_builder();
        self.spawn(b)
            .expect("failed to spawn a thread in thread pool");
    }

    pub fn spawn(&self, thread_builder: thread::Builder) -> io::Result<thread::JoinHandle<()>> {
        let timeout = self.timeout;
        let channel = self.channel.clone();

        let worker = move || {
            let mut rx = channel.consume();
            loop {
                rx = match rx.tasks.pop_front() {
                    Some(task) => {
                        drop(rx);
                        task.run();

                        let mut rx = channel.consume();
                        rx.count -= 1;
                        rx
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

    pub fn thread_builder(&self) -> thread::Builder {
        let mut thread = thread::Builder::new();
        if let Some(size) = self.get_stack_size() {
            thread = thread.stack_size(size);
        }
        let name = (self.name)(self.thread_count());
        if !name.is_empty() {
            thread = thread.name(name);
        }
        thread
    }
}
