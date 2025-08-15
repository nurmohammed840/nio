use crate::runtime::Task;
use crossbeam_channel::{unbounded as channel, Receiver, Sender};
// use std::sync::mpsc::{channel, Receiver, Sender};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU8, AtomicUsize, Ordering},
        Arc,
    },
};

pub struct Scheduler {
    workers: Arc<Batch<Worker>>,
}

struct Batch<T> {
    next_batch: AtomicU8,
    workers: Box<[T]>,
}

const BATCH_SIZE: usize = 8;

impl<T> Batch<T> {
    fn new(workers: Box<[T]>) -> Self {
        Self {
            next_batch: AtomicU8::new(0),
            workers,
        }
    }

    #[inline]
    fn next_batch(&self) -> &[T] {
        let len = self.workers.len();
        if len <= BATCH_SIZE {
            &self.workers
        } else {
            let no = self.next_batch.fetch_add(1, Ordering::Relaxed) as usize;
            let next = (no * BATCH_SIZE) % len.next_multiple_of(BATCH_SIZE);
            let end = (next + BATCH_SIZE).min(len);
            unsafe { self.workers.get_unchecked(next..end) }
        }
    }
}

struct Worker {
    len: Length,
    tx: Sender<Task>,
}

pub struct TaskQueue {
    len: Length,
    rx: Receiver<Task>,
    defer: VecDeque<Task>,
    defer_count: usize,
}

impl TaskQueue {
    pub fn registered(&mut self) {}

    #[inline]
    pub fn fetch(&mut self) -> Option<Task> {
        if self.defer_count > 0 {
            self.defer_count -= 1;
            return self.defer.pop_front();
        }

        if let task @ Some(_) = self.rx.try_recv().ok() {
            self.len.dec();
            return task;
        }

        if let task @ Some(_) = self.defer.pop_front() {
            self.defer_count = self.defer.len();
            return task;
        }
        let task = self.rx.recv().ok();
        self.len.dec();
        task
    }

    #[inline]
    pub fn add(&mut self, task: Task) {
        self.defer.push_back(task);
    }

    pub fn deregister(self) {}
}

impl Scheduler {
    pub fn new(worker_count: usize) -> (Self, Vec<TaskQueue>) {
        assert!(worker_count > 0);

        let init_capacity = 256;
        let mut queues = vec![];
        let workers: Box<_> = (0..worker_count)
            .map(|_| {
                let len = Length::default();
                let (tx, rx) = channel();

                queues.push(TaskQueue {
                    len: len.clone(),
                    defer: VecDeque::with_capacity(init_capacity),
                    defer_count: 0,
                    rx,
                });

                Worker { len, tx }
            })
            .collect();

        let scheduler = Self {
            workers: Batch::new(workers).into(),
        };
        (scheduler, queues)
    }

    fn least_loaded_worker(&self) -> &Worker {
        unsafe { min_by_key(self.workers.next_batch(), |w| w.len.get()) }
    }

    pub fn schedule(&self, task: Task) {
        let sender = self.least_loaded_worker();
        sender.tx.send(task).unwrap();
        sender.len.inc();
    }

    #[inline]
    pub fn spawn(&self, task: Task) {
        self.schedule(task);
    }
}

#[derive(Default, Clone, Debug)]
pub struct Length(Arc<AtomicUsize>);

impl Length {
    #[inline]
    pub fn get(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }

    #[inline]
    fn inc(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    fn dec(&self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

impl Clone for Scheduler {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            workers: Arc::clone(&self.workers),
        }
    }
}

#[inline]
unsafe fn min_by_key<T, B: Ord>(data: &[T], f: fn(&T) -> B) -> &T {
    debug_assert!(!data.is_empty());

    let mut val_x = data.get_unchecked(0);
    let mut x = f(val_x);

    for val_y in data.get_unchecked(1..) {
        let y = f(val_y);
        if x < y {
            break;
        }
        x = y;
        val_x = val_y;
    }
    val_x
}
