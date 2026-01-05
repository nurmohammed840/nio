use crate::driver::{self, Driver};

use super::*;
use std::{
    io,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

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
    wakers: Box<[driver::Waker]>,
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
    pub fn wake(&self, id: WorkerId) {
        unsafe { self.wakers.get_unchecked(id.get()) }.wake();
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
    pub fn new(count: u8) -> io::Result<(Self, Box<[Driver]>)> {
        let mut drivers = Vec::with_capacity(count as usize);
        let mut wakers = Vec::with_capacity(count as usize);

        for _ in (0..count) {
            let (driver, waker) = Driver::with_capacity(1024)?;
            drivers.push(driver);
            wakers.push(waker);
        }

        Ok((
            Self {
                wakers: wakers.into_boxed_slice(),
                task_counters: (0..count).map(|_| TaskCounter::new()).collect(),
                shared_queues: (0..count).map(|_| SegQueue::new()).collect(),
            },
            drivers.into_boxed_slice(),
        ))
    }

    pub fn job(context: Rc<LocalContext>, tick: u32, mut driver: Driver) {
        debug_assert_ne!(tick, 0);
        context.clone().init();

        let task_counter = context.task_counter();

        loop {
            let mut tick = Tick(tick);
            'event_interval: while tick.has_steps_left() {
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
                            break 'event_interval;
                        }
                    }
                }
            }

            let (timers, timeout) = unsafe {
                context.timers(|timer| {
                    let now = timer.clock.now();
                    let done = timer.fetch(now);
                    let timeout = if done.is_some() {
                        Some(Duration::ZERO)
                    } else {
                        timer.next_timeout(now)
                    };
                    (done, timeout)
                })
            };

            if let Some(timers) = timers {
                timers.notify_all();
            }

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

            for ev in events {
                if Driver::has_woken(ev) {
                    continue;
                }
            }
        }
    }
}

struct Tick(u32);

impl Tick {
    fn has_steps_left(&self) -> bool {
        self.0 > 0
    }
    fn step(&mut self) {
        self.0 -= 1;
    }
}
