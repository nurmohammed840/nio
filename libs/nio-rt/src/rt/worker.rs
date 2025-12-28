use super::*;
use std::sync::Arc;

use crossbeam_queue::SegQueue;

use local_context::LocalContext;
use task::{Status, Task};
use task_counter::TaskCounter;

pub type SharedQueue = SegQueue<Task>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkerId(u8);

impl WorkerId {
    pub unsafe fn new(id: u8) -> WorkerId {
        WorkerId(id)
    }

    #[inline]
    pub fn get(self) -> usize {
        self.0 as usize
    }
}

pub struct Workers {
     task_counters: Box<[TaskCounter]>,
     shared_queues: Box<[SharedQueue]>,
}

impl Workers {
    pub fn create_id(&self, id: u8) -> WorkerId {
        if self.task_counters.get(id as usize).is_none() {
            panic!(
                "invalid worker id {id}:, (valid range: 0..{})",
                self.task_counters.len()
            );
        }
        unsafe { WorkerId::new(id) }
    }

    #[inline]
    pub fn task_counter(&self, id: WorkerId) -> &TaskCounter {
        unsafe { self.task_counters.get_unchecked(id.get()) }
    }

    #[inline]
    pub fn shared_queue(&self, id: WorkerId) -> &SharedQueue {
        unsafe { self.shared_queues.get_unchecked(id.get()) }
    }

    pub fn least_loaded_worker_index(&self) -> WorkerId {
        unsafe {
            // Safety: `task_counters` is not empty
            let id = crate::utils::min_index_by_key(&self.task_counters, |counter| {
                counter.load().total()
            });
            debug_assert!(self.task_counters.get(id).is_some());
            WorkerId::new(id as u8)
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

    pub fn job(id: WorkerId, tick: u32, runtime_ctx: Arc<RuntimeContext>) {
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
