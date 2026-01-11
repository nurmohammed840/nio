use crate::driver::{self, Driver};

use super::*;
use std::{
    io,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
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

pub struct Notifier {
    waker: driver::Waker,
    state: AtomicUsize,
}

impl Notifier {
    const RESET: usize = 0;
    const NOTIFIED: usize = 1;

    fn new(waker: driver::Waker) -> Self {
        Self {
            waker,
            state: AtomicUsize::new(Notifier::RESET),
        }
    }

    pub fn notify_once(&self) {
        if self.state.swap(Notifier::NOTIFIED, Ordering::AcqRel) == Notifier::RESET {
            let _result = self.waker.wake();
            debug_assert!(_result.is_ok(), "{_result:?}")
        }
    }

    pub fn accept_notify_once(&self) {
        self.state.store(Notifier::RESET, Ordering::Release);
    }
}

pub struct Workers {
    notifiers: Box<[Notifier]>,
    shared_queues: Box<[SharedQueue]>,
    pub(crate) task_counters: Box<[TaskCounter]>,
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

    pub fn notifier(&self, id: WorkerId) -> &Notifier {
        unsafe { self.notifiers.get_unchecked(id.get()) }
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
        let mut notifier = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let (driver, waker) = Driver::with_capacity(1024)?;
            drivers.push(driver);
            notifier.push(Notifier::new(waker));
        }

        Ok((
            Self {
                notifiers: notifier.into_boxed_slice(),
                task_counters: (0..count).map(|_| TaskCounter::new()).collect(),
                shared_queues: (0..count).map(|_| SegQueue::new()).collect(),
            },
            drivers.into_boxed_slice(),
        ))
    }

    pub fn job(context: Rc<LocalContext>, tick: u32, mut driver: Driver) {
        context.clone().init();

        let notifier = context.notifier();
        let task_counter = context.task_counter();

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
                        let counter = task_counter.decrease_local();
                        context.move_tasks_from_shared_to_local_queue(counter);
                    }
                }
            }

            let mut local_queue_is_empty = unsafe { context.local_queue(|q| q.is_empty()) };
            
            if local_queue_is_empty {
                // Accept notification from other threads.
                notifier.accept_notify_once();
            }

            let counter = task_counter.load();
            if counter.shared_queue_has_data() {
                context.move_tasks_from_shared_to_local_queue(counter);
                local_queue_is_empty = false
            }

            let (expired, timeout) = unsafe {
                context.timers(|timer| {
                    let now = timer.clock.now();
                    let timers_expired = timer.fetch(now);

                    let timeout = if !local_queue_is_empty || timers_expired.is_some() {
                        // Do not sleep; We have more work to do.
                        Some(Duration::ZERO)
                    } else {
                        // No immediate work; Sleep until the next timer fires,
                        // or until woken by an I/O event or another thread send more task.
                        timer.next_timeout(now)
                    };
                    (timers_expired, timeout)
                })
            };

            if let Some(timers) = expired {
                timers.notify_all();
            }

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
