use crate::{
    LocalContext, RuntimeContext,
    driver::{self, Driver},
};
use nio_task::Status;
use std::{io, rc::Rc, sync::Arc, time::Duration};

pub struct EventLoop {
    tick: u32,
    driver: Driver,
    local_ctx: Rc<LocalContext>,
}

impl EventLoop {
    pub(crate) fn new(
        id: u8,
        driver: driver::Driver,
        runtime_ctx: Arc<RuntimeContext>,
        tick: u32,
        local_queue_cap: usize,
    ) -> Self {
        let worker_id = runtime_ctx.workers.id(id);
        let io_registry = driver.registry_owned().unwrap();
        let local_ctx = LocalContext::new(worker_id, local_queue_cap, runtime_ctx, io_registry);
        local_ctx.clone().init();
        EventLoop {
            tick,
            driver,
            local_ctx,
        }
    }

    pub(crate) fn run(&mut self) {
        let task_queue = self.local_ctx.task_queue();

        loop {
            for _ in 0..self.tick {
                let Some(task) = (unsafe { self.local_ctx.local_queue(|q| q.pop_front()) }) else {
                    break;
                };
                match task.poll() {
                    Status::Yielded(task) => {
                        unsafe { self.local_ctx.local_queue(|q| q.push_back(task)) };
                    }
                    Status::Pending | Status::Complete(_) => {
                        let counter = task_queue.decrease_local();
                        self.local_ctx
                            .move_tasks_from_shared_to_local_queue(counter);
                    }
                }
            }

            let expired_timers = unsafe {
                self.local_ctx
                    .timers(|timer| timer.fetch(timer.clock.now()))
            };
            expired_timers.notify_all();

            let mut local_queue_is_empty = unsafe { self.local_ctx.local_queue(|q| q.is_empty()) };

            let counter = if local_queue_is_empty {
                // Accept notification from other threads.
                let (_notify_flag_removed, state) =
                    task_queue.accept_notify_once_if_shared_queue_is_empty();

                #[cfg(feature = "metrics")]
                if _notify_flag_removed {
                    self.ctx
                        .runtime_ctx
                        .measurement
                        .queue_drained(self.ctx.worker_id.get());
                }

                state
            } else {
                task_queue.load()
            };

            if counter.shared_queue_has_data() {
                self.local_ctx
                    .move_tasks_from_shared_to_local_queue(counter);
                local_queue_is_empty = false
            }

            let timeout = unsafe {
                self.local_ctx.timers(|timer| {
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
            let events = match self.driver.poll(timeout) {
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
