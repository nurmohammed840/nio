use super::*;
use std::sync::Arc;

use crossbeam_queue::SegQueue;

use local_context::LocalContext;
use task::{Status, Task};
use task_counter::TaskCounter;

pub struct Worker {
    pub id: u8,
    pub task_counter: TaskCounter,
    pub shared_queue: SegQueue<Task>,
}

impl Worker {
    pub fn new(id: u8) -> Self {
        Self {
            id,
            task_counter: TaskCounter::new(),
            shared_queue: SegQueue::new(),
        }
    }

    pub fn job(id: u8, tick: u32, runtime_ctx: Arc<RuntimeContext>) {
        let context = LocalContext::new(id, 512, runtime_ctx);
        context.clone().init();

        let worker = context.current();

        let mut tick = Tick(tick);
        while !tick.is_complete() {
            match unsafe { context.local_queue() }.pop_front() {
                Some(task) => match task.poll() {
                    Status::Yielded(task) => {
                        unsafe { context.local_queue() }.push_back(task);
                    }
                    Status::Pending | Status::Complete(_) => {
                        tick.step();
                        let counter = worker.task_counter.decrease_local();
                        unsafe { context.move_tasks_from_shared_to_local_queue(counter) };
                    }
                },
                None => {
                    let counter = worker.task_counter.load();
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
