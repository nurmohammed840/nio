use crate::RuntimeConfig;
use crate::blocking::BlockingTask;
use std::{cell::UnsafeCell, collections::VecDeque, rc::Rc, thread};

use nio_task::{Status, Task};
use nio_threadpool::ThreadPool;

impl RuntimeConfig {
    pub fn rt(mut self) -> Runtime {
        let threadpool = ThreadPool::new()
            .max_threads_limit(self.max_blocking_threads)
            .stack_size(self.thread_stack_size)
            .timeout(self.thread_timeout)
            .name(self.thread_name.take().unwrap());

        for id in 0..self.worker_threads {
            let join_handle = self
                .create_thread(id)
                .spawn(|| {
                    let local_queue = LocalQueue::new(512);
                    Context::init(local_queue.clone());

                    while let Some(task) = unsafe { local_queue.get_mut() }.pop_front() {
                        match task.poll() {
                            Status::Yielded(task) => {
                                unsafe { local_queue.get_mut() }.push_back(task);
                            }
                            Status::Pending => {}
                            Status::Complete => {}
                        }
                    }
                })
                .expect(&format!("failed to spawn worker thread: {id}"));

            Worker { join_handle };
        }

        Runtime {
            _workers: Box::new([]),
            _threadpool: threadpool,
        }
    }

    fn create_thread(&self, id: usize) -> thread::Builder {
        let mut thread = thread::Builder::new();
        if let Some(size) = self.worker_stack_size {
            thread = thread.stack_size(size);
        }
        let name = (self.worker_name)(id);
        if !name.is_empty() {
            thread = thread.name(name);
        }
        thread
    }
}

pub struct Worker {
    join_handle: std::thread::JoinHandle<()>,
}

pub struct Runtime {
    _workers: Box<[Worker]>,
    _threadpool: ThreadPool<BlockingTask>,
}

// -------------------------------------------------------------------------------------------

struct Context {
    local_queue: LocalQueue,
}

#[derive(Clone)]
struct LocalQueue {
    inner: Rc<UnsafeCell<VecDeque<Task>>>,
}

impl LocalQueue {
    fn new(cap: usize) -> Self {
        Self {
            inner: Rc::new(UnsafeCell::new(VecDeque::with_capacity(cap))),
        }
    }
    // Safety: the caller must ensure that there are no references alive when this is called.
    unsafe fn get_mut(&self) -> &mut VecDeque<Task> {
        unsafe { &mut *self.inner.get() }
    }
}

impl Context {
    fn init(queue: LocalQueue) {
        CONTEXT.with(|ctx| unsafe { *ctx.get() = Some(Self { local_queue: queue }) });
    }
    pub fn with<F, R>(f: F) -> R
    where
        F: FnOnce(&Self) -> R,
    {
        CONTEXT.with(|ctx| unsafe { f((*ctx.get()).as_ref().expect("no `Nio` runtime found")) })
    }
}

thread_local! {
    static CONTEXT: UnsafeCell<Option<Context>> = const { UnsafeCell::new(None) };
}
