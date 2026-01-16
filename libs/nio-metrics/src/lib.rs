use std::{
    any::Any,
    fmt::Debug,
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

pub trait Measurement: Debug + Send + Sync + Any {
    fn init(&mut self, _worker_threads: usize) {}
    fn queue_drained(&self, _worker_id: usize) {}
    fn queue_notified(&self, _worker_id: usize) {}
}

#[derive(Debug, Default)]
struct TaskQueue {
    drained: AtomicUsize,
    notified: AtomicUsize,
}

#[derive(Debug, Default)]
pub struct SimpleMeasurement {
    workers: Vec<TaskQueue>,
}

impl Measurement for SimpleMeasurement {
    fn init(&mut self, worker_threads: usize) {
        self.workers = (0..worker_threads).map(|_| TaskQueue::default()).collect();
    }

    fn queue_drained(&self, worker_id: usize) {
        self.workers[worker_id].drained.fetch_add(1, Relaxed);
    }

    fn queue_notified(&self, worker_id: usize) {
        self.workers[worker_id].notified.fetch_add(1, Relaxed);
    }
}

impl SimpleMeasurement {
    pub fn new() -> SimpleMeasurement {
        SimpleMeasurement { workers: vec![] }
    }
}
