use nio_task::{JoinHandle, Task};
use std::sync::Arc;

use crate::rt::context::RuntimeContext;

pub struct Scheduler {
    runtime_ctx: Arc<RuntimeContext>,
}

impl nio_task::Scheduler for Scheduler {
    fn schedule(&self, task: Task) {
        self.runtime_ctx.send_task_to_least_loaded_worker(task);
    }
}

impl Scheduler {
    pub fn spawn<F>(ctx: Arc<RuntimeContext>, future: F) -> (Task, JoinHandle<F::Output>)
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        unsafe { Task::new_unchecked((), future, Scheduler { runtime_ctx: ctx }) }
    }
}
