use super::*;
use std::sync::Arc;

use crossbeam_queue::SegQueue;

use local_context::LocalContext;
use task::{Status, Task};
use task_counter::TaskCounter;

pub type SharedQueue = SegQueue<Task>;

pub struct Workers {
    pub task_counters: Box<[TaskCounter]>,
    pub shared_queues: Box<[SharedQueue]>,
}

impl Workers {
    pub fn least_loaded_worker_index(&self) -> usize {
        // Safety: `task_counters` is not empty
        unsafe {
            crate::utils::min_index_by_key(&self.task_counters, |counter| counter.load().total())
        }
    }
}

impl Workers {
    pub fn new(count: u8) -> Self {
        Self {
            task_counters: (0..count).map(|_| TaskCounter::new()).collect(),
            shared_queues: (0..count).map(|_| SegQueue::new()).collect(),
        }
    }

    pub fn job(id: u8, tick: u32, runtime_ctx: Arc<RuntimeContext>) {
        let context = LocalContext::new(id, 512, runtime_ctx);
        context.clone().init();

        let task_counter = context.task_counter();

        let mut tick = Tick(tick);
        while !tick.is_complete() {
            match unsafe { context.local_queue() }.pop_front() {
                Some(task) => match task.poll() {
                    Status::Yielded(task) => {
                        unsafe { context.local_queue() }.push_back(task);
                    }
                    Status::Pending | Status::Complete(_) => {
                        tick.step();
                        let counter = task_counter.decrease_local();
                        unsafe { context.move_tasks_from_shared_to_local_queue(counter) };
                    }
                },
                None => {
                    let counter = task_counter.load();
                    if counter.shared_queue_has_data() {
                        unsafe { context.move_tasks_from_shared_to_local_queue(counter) };
                    } else {
                        break;
                    }
                }
            }
        }
    }
}

struct Tick(pub u32);

impl Tick {
    fn is_complete(&self) -> bool {
        self.0 == 0
    }
    fn step(&mut self) {
        self.0 -= 1;
    }
}
