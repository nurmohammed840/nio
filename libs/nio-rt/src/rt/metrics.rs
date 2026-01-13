use std::sync::Arc;

use crate::RuntimeContext;
pub use crate::rt::task_counter::Counter;

pub struct RuntimeMetrics {
    pub(crate) ctx: Arc<RuntimeContext>,
}

impl RuntimeMetrics {
    pub fn num_workers(&self) -> usize {
        self.ctx.workers.task_counters.len()
    }

    pub fn task_counts(&self) -> Vec<Counter> {
        let mut arr = Vec::with_capacity(self.num_workers());
        for task_counter in &self.ctx.workers.task_counters {
            arr.push(task_counter.load());
        }
        arr
    }
}
