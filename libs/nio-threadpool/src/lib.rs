use mpmc_channel::MPMC;
use std::{
    collections::VecDeque,
    io,
    num::NonZero,
    panic::{self, AssertUnwindSafe},
    sync::Arc,
    thread,
    time::Duration,
};

type Channel<Task> = Arc<MPMC<Queue<Task>>>;

struct Queue<Task> {
    tasks: VecDeque<Task>,
    count: usize,
    workers: usize,
}

pub struct ThreadPool<Task: Runnable> {
    channel: Channel<Task>,

    // ----- config -----
    timeout: Option<Duration>,
    stack_size: Option<NonZero<usize>>,
    name: Box<dyn Fn(usize) -> String + Send + Sync>,
}

impl<T: Runnable> Default for ThreadPool<T> {
    fn default() -> Self {
        Self {
            channel: Arc::new(MPMC::new(Queue {
                tasks: VecDeque::with_capacity(64),
                count: 0,
                workers: 0,
            })),

            timeout: Some(Duration::from_secs(7)),
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

    pub fn stack_size(mut self, stack_size: usize) -> Self {
        self.stack_size = NonZero::new(stack_size);
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

    // -------------------------------------------

    pub fn thread_count(&self) -> usize {
        self.channel.consume().workers
    }

    pub fn execute(&self, task: Task) {
        let spawn_needed = {
            let mut q = self.channel.produce();
            q.tasks.push_back(task);
            q.count += 1;

            let all_busy = q.count > q.workers;
            if all_busy {
                q.workers += 1;
            }

            q.notify_one();
            all_busy
        };

        if spawn_needed {
            if let Err(err) = self.spawn_reserved(self.thread_builder()) {
                let stranded = {
                    let mut q = self.channel.produce();
                    q.workers -= 1;
                    q.workers == 0
                };
                // Panic only after the lock is released, or the mutex gets poisoned.
                // Skip it if other workers exist: they will run the task.
                if stranded {
                    panic!("failed to spawn a thread in thread pool: {err}");
                }
            }
        }
    }

    pub fn spawn(&self, builder: thread::Builder) -> io::Result<thread::JoinHandle<()>> {
        self.channel.produce().workers += 1;
        self.spawn_reserved(builder).inspect_err(|_| {
            self.channel.produce().workers -= 1;
        })
    }

    fn spawn_reserved(
        &self,
        thread_builder: thread::Builder,
    ) -> io::Result<thread::JoinHandle<()>> {
        let timeout = self.timeout;
        let channel = self.channel.clone();

        let worker = move || {
            let mut rx = channel.consume();
            loop {
                rx = match rx.tasks.pop_front() {
                    Some(task) => {
                        drop(rx);
                        let _ = panic::catch_unwind(AssertUnwindSafe(|| task.run()));

                        rx = channel.consume();
                        rx.count -= 1;
                        rx
                    }
                    None => match timeout {
                        None => rx.wait(),
                        Some(dur) => match rx.wait_timeout(dur) {
                            Ok(rx) => rx,
                            Err(_) => {
                                rx = channel.consume();
                                if rx.tasks.is_empty() {
                                    rx.workers -= 1;
                                    break;
                                }
                                rx
                            }
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
