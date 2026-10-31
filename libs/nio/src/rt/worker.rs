use crate::{
    driver::{self, Driver},
    utils::find_index_of_lowest,
};

use super::*;
use std::io;

use crossbeam_queue::SegQueue;

use task::Task;
use task_queue::TaskQueue;

pub type SharedQueue = SegQueue<Task>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkerId(u8);

impl WorkerId {
    #[inline]
    pub fn get(self) -> usize {
        self.0 as usize
    }
}

pub struct Workers {
    notifiers: Box<[driver::Waker]>,
    shared_queues: Box<[SharedQueue]>,
    pub(crate) task_queues: Box<[TaskQueue]>,
    pub(crate) min_tasks_per_worker: u64,
}

impl Workers {
    pub fn id(&self, id: u8) -> WorkerId {
        if self.task_queues.get(id as usize).is_none() {
            panic!(
                "invalid worker id {id}:, (valid range: 0..{})",
                self.task_queues.len()
            );
        }
        WorkerId(id)
    }

    pub fn least_loaded_worker(&self) -> WorkerId {
        unsafe {
            // Safety: `task_counters` is not empty
            let id =
                find_index_of_lowest(&self.task_queues, self.min_tasks_per_worker, |counter| {
                    counter.load().total()
                });
            debug_assert!(self.task_queues.get(id).is_some());
            WorkerId(id as u8)
        }
    }

    pub fn notifier(&self, id: WorkerId) -> &driver::Waker {
        unsafe { self.notifiers.get_unchecked(id.get()) }
    }

    #[inline]
    pub fn task_queue(&self, id: WorkerId) -> &TaskQueue {
        unsafe { self.task_queues.get_unchecked(id.get()) }
    }

    #[inline]
    pub fn shared_queue(&self, id: WorkerId) -> &SharedQueue {
        unsafe { self.shared_queues.get_unchecked(id.get()) }
    }
}

impl Workers {
    pub fn new(count: u8, min_tasks_per_worker: u64) -> io::Result<(Self, Box<[Driver]>)> {
        let mut drivers = Vec::with_capacity(count as usize);
        let mut notifier = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let (driver, waker) = Driver::with_capacity(1024)?;
            drivers.push(driver);
            notifier.push(waker);
        }

        Ok((
            Workers {
                min_tasks_per_worker,
                notifiers: notifier.into_boxed_slice(),
                task_queues: (0..count).map(|_| TaskQueue::new()).collect(),
                shared_queues: (0..count).map(|_| SegQueue::new()).collect(),
            },
            drivers.into_boxed_slice(),
        ))
    }
}
