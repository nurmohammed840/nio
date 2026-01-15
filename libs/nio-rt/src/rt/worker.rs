use crate::{
    driver::{self, Driver},
    utils::min_index_by_key,
};

use super::*;
use std::{io, rc::Rc, time::Duration};

use crossbeam_queue::SegQueue;

use context::LocalContext;
use task::{Status, Task};
use task_queue::TaskQueue;

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
    notifiers: Box<[driver::Waker]>,
    shared_queues: Box<[SharedQueue]>,
    pub(crate) task_queues: Box<[TaskQueue]>,
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
            let id = min_index_by_key(&self.task_queues, |counter| counter.load().total());
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
    pub fn new(count: u8) -> io::Result<(Self, Box<[Driver]>)> {
        let mut drivers = Vec::with_capacity(count as usize);
        let mut notifier = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let (driver, waker) = Driver::with_capacity(1024)?;
            drivers.push(driver);
            notifier.push(waker);
        }

        Ok((
            Self {
                notifiers: notifier.into_boxed_slice(),
                task_queues: (0..count).map(|_| TaskQueue::new()).collect(),
                shared_queues: (0..count).map(|_| SegQueue::new()).collect(),
            },
            drivers.into_boxed_slice(),
        ))
    }

    pub fn job(context: Rc<LocalContext>, tick: u32, mut driver: Driver) {
        context.clone().init();

        let task_queue = context.task_queue();

        loop {
            for _ in 0..tick {
                let Some(task) = (unsafe { context.local_queue(|q| q.pop_front()) }) else {
                    break;
                };
                match task.poll() {
                    Status::Yielded(task) => {
                        unsafe { context.local_queue(|q| q.push_back(task)) };
                    }
                    Status::Pending | Status::Complete(_) => {
                        let counter = task_queue.decrease_local();
                        context.move_tasks_from_shared_to_local_queue(counter);
                    }
                }
            }

            let expired_timers = unsafe { context.timers(|timer| timer.fetch(timer.clock.now())) };
            expired_timers.notify_all();

            let mut local_queue_is_empty = unsafe { context.local_queue(|q| q.is_empty()) };

            let counter = if local_queue_is_empty {
                // Accept notification from other threads.
                task_queue.accept_notify_once_if_shared_queue_is_empty()
            } else {
                task_queue.load()
            };

            if counter.shared_queue_has_data() {
                context.move_tasks_from_shared_to_local_queue(counter);
                local_queue_is_empty = false
            }

            let timeout = unsafe {
                context.timers(|timer| {
                    if local_queue_is_empty {
                        // No immediate work; Sleep until the next timer fires,
                        // or until woken by an I/O event or another thread send more task.
                        return timer.next_timeout(timer.clock.current());
                    }
                    // Do not sleep; We have more work to do.
                    Some(Duration::ZERO)
                })
            };

            // `driver.poll` method clear wake up notifications.
            let events = match driver.poll(timeout) {
                Ok(events) => events,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                #[cfg(target_os = "wasi")]
                Err(e) if e.kind() == io::ErrorKind::InvalidInput => {
                    // In case of wasm32_wasi this error happens, when trying to poll without subscriptions
                    // just return from the park, as there would be nothing, which wakes us up.
                    continue;
                }
                Err(e) => panic!("unexpected error when polling the I/O driver: {e:?}"),
            };

            for event in events {
                if Driver::has_woken(event) {
                    continue;
                }
                let ptr = driver::IoWaker::from(event.token().0);
                unsafe { (*ptr).notify(event) };
            }
        }
    }
}
