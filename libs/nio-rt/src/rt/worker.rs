use super::*;
use std::{rc::Rc, sync::Arc};

use crossbeam_queue::SegQueue;

use context::LocalContext;
use task::{Status, Task};
use task_counter::TaskCounter;

pub type SharedQueue = SegQueue<Task>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkerId(u8);

impl WorkerId {
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
    pub fn id(&self, id: u8) -> WorkerId {
        if self.task_counters.get(id as usize).is_none() {
            panic!(
                "invalid worker id {id}:, (valid range: 0..{})",
                self.task_counters.len()
            );
        }
        WorkerId(id)
    }

    pub fn least_loaded_worker(&self) -> WorkerId {
        unsafe {
            // Safety: `task_counters` is not empty
            let id = crate::utils::min_index_by_key(&self.task_counters, |counter| {
                counter.load().total()
            });
            debug_assert!(self.task_counters.get(id).is_some());
            WorkerId(id as u8)
        }
    }

    #[inline]
    pub fn task_counter(&self, id: WorkerId) -> &TaskCounter {
        unsafe { self.task_counters.get_unchecked(id.get()) }
    }

    #[inline]
    pub fn shared_queue(&self, id: WorkerId) -> &SharedQueue {
        unsafe { self.shared_queues.get_unchecked(id.get()) }
    }
}

impl Workers {
    pub fn new(count: u8) -> Self {
        Self {
            task_counters: (0..count).map(|_| TaskCounter::new()).collect(),
            shared_queues: (0..count).map(|_| SegQueue::new()).collect(),
        }
    }

    pub fn job(context: Rc<LocalContext>, tick: u32) {
        context.clone().init();
        
        let task_counter = context.task_counter();

        let mut tick = Tick(tick);
        while !tick.is_complete() {
            match unsafe { context.local_queue(|q| q.pop_front()) } {
                Some(task) => match task.poll() {
                    Status::Yielded(task) => {
                        unsafe { context.local_queue(|q| q.push_back(task)) };
                    }
                    Status::Pending | Status::Complete(_) => {
                        tick.step();
                        let counter = task_counter.decrease_local();
                        context.move_tasks_from_shared_to_local_queue(counter);
                    }
                },
                None => {
                    let counter = task_counter.load();
                    if counter.shared_queue_has_data() {
                        context.move_tasks_from_shared_to_local_queue(counter);
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
