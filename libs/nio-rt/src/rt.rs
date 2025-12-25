#![allow(unused)]

use crate::RuntimeConfig;
use crate::blocking::BlockingTask;
use std::{
    cell::UnsafeCell,
    collections::VecDeque,
    rc::Rc,
    sync::{Arc, atomic::AtomicU64},
    thread,
};

use crossbeam_queue::SegQueue;
use nio_task::{JoinHandle, Status, Task};
use nio_threadpool::ThreadPool;

impl RuntimeConfig {
    pub fn rt(mut self) -> Runtime {
        let context = Arc::new(RuntimeContext {
            workers: (0..self.worker_threads).map(|_| Worker::new()).collect(),
            threadpool: ThreadPool::new()
                .max_threads_limit(self.max_blocking_threads)
                .stack_size(self.thread_stack_size)
                .timeout(self.thread_timeout)
                .name(self.thread_name.take().unwrap()),
        });
        Runtime {
            context,
            config: self,
        }
    }
}

pub struct Runtime {
    config: RuntimeConfig,
    context: Arc<RuntimeContext>,
}

impl Runtime {
    fn create_thread(&self, id: u8) -> thread::Builder {
        let mut thread = thread::Builder::new();
        if let Some(size) = self.config.worker_stack_size {
            thread = thread.stack_size(size);
        }
        let name = (self.config.worker_name)(id);
        if !name.is_empty() {
            thread = thread.name(name);
        }
        thread
    }

    pub fn block_on(self) {
        for id in 1..self.config.worker_threads {
            let runtime = self.context.clone();
            self.create_thread(id)
                .spawn(move || worker(id, runtime))
                .expect(&format!("failed to spawn worker thread: {id}"));
        }
        drop(self.config);

        worker(0, self.context)
    }
}

fn worker(id: u8, runtime_ctx: Arc<RuntimeContext>) {
    let local_queue: LocalQueue = LocalQueue::new(512);
    Context::init(local_queue.clone(), id, runtime_ctx);

    while let Some(task) = unsafe { local_queue.get_mut() }.pop_front() {
        match task.poll() {
            Status::Yielded(task) => {
                unsafe { local_queue.get_mut() }.push_back(task);
            }
            Status::Pending => {}
            Status::Complete => {}
        }
    }
}

struct TaskCounter {
    counter: AtomicU64,
}

impl TaskCounter {
    fn new() -> Self {
        Self {
            counter: AtomicU64::new(0),
        }
    }
}

pub struct Worker {
    task_counter: TaskCounter,
    shared_queue: SegQueue<Task>,
}

impl Worker {
    pub fn new() -> Self {
        Self {
            task_counter: TaskCounter::new(),
            shared_queue: SegQueue::new(),
        }
    }
}

pub struct RuntimeContext {
    workers: Box<[Worker]>,
    threadpool: ThreadPool<BlockingTask>,
}

impl RuntimeContext {
    pub fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (task, join) = nio_task::BlockingTask::new(f);
        self.threadpool.execute(BlockingTask { task });
        join
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (task, join) = nio_task::Task::new(future, |_| {});
        self.workers[0].shared_queue.push(task);
        join
    }
}

struct Context {
    worker_id: u8,
    local_queue: LocalQueue,
    runtime_ctx: Arc<RuntimeContext>,
}

impl Context {
    fn current(&self) -> &Worker {
        unsafe {
            self.runtime_ctx
                .workers
                .get_unchecked(self.worker_id as usize)
        }
    }
}

// -------------------------------------------------------------------------------------------

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
    fn init(local_queue: LocalQueue, id: u8, runtime_ctx: Arc<RuntimeContext>) {
        CONTEXT.with(|ctx| unsafe {
            *ctx.get() = Some(Self {
                local_queue,
                runtime_ctx,
                worker_id: id,
            })
        });
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
