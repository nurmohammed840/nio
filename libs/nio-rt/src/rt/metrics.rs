use std::sync::Arc;

use crate::RuntimeContext;
pub use crate::rt::task_queue::{Counter, TaskQueue};

pub struct RuntimeMetrics {
    pub(crate) ctx: Arc<RuntimeContext>,
}

impl RuntimeMetrics {
    pub fn num_workers(&self) -> usize {
        self.ctx.workers.task_queues.len()
    }

    pub fn task_counts(&self) -> impl Iterator<Item = Counter> {
        self.ctx.workers.task_queues.iter().map(TaskQueue::load)
    }
}
