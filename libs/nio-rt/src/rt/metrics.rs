use std::sync::Arc;

use crate::RuntimeContext;
use crate::rt::task_queue::TaskQueue;

pub use crate::rt::task_queue::Counter;
pub use nio_metrics::Measurement;

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

    #[cfg(feature = "metrics")]
    pub fn measurement(&self) -> &dyn Measurement {
        &*self.ctx.measurement
    }

    #[cfg(not(feature = "metrics"))]
    pub fn measurement(&self) -> &dyn Measurement {
        &NoMeasurement
    }

    pub fn measurement_as<T: 'static>(&self) -> Option<&T> {
        #[cfg(not(feature = "metrics"))]
        return None;
        #[cfg(feature = "metrics")]
        {
            let measurement: &dyn Any = &self.ctx.measurement;
            measurement.downcast_ref::<T>()
        }
    }
}

#[derive(Debug)]
pub(crate) struct NoMeasurement;
impl Measurement for NoMeasurement {}

#[cfg(test)]
mod tests {
    use super::*;
    fn _test_measurement_as_any(metrics: RuntimeMetrics) {
        let _: &dyn std::any::Any = metrics.measurement();
    }
}
