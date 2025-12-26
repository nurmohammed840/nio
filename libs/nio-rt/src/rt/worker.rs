use super::*;
use std::sync::Arc;

use crossbeam_queue::SegQueue;
use nio_task::{Status, Task};

use local_context::LocalContext;
use local_queue::LocalQueue;
use task_counter::TaskCounter;

pub struct Worker {
    pub task_counter: TaskCounter,
    pub shared_queue: SegQueue<Task>,
}

impl Worker {
    pub fn new() -> Self {
        Self {
            task_counter: TaskCounter::new(),
            shared_queue: SegQueue::new(),
        }
    }

    pub fn job(id: u8, runtime_ctx: Arc<RuntimeContext>) {
        let local_queue: LocalQueue = LocalQueue::new(512);
        LocalContext::init(local_queue.clone(), id, runtime_ctx);

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
}
