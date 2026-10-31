use super::*;
use task::*;

pub struct RuntimeContext {
    pub(crate) workers: Workers,
    pub(crate) threadpool: ThreadPool<BlockingTask>,

    #[cfg(feature = "metrics")]
    pub(crate) measurement: Box<dyn metrics::Measurement>,
}

impl RuntimeContext {
    pub(crate) fn with<F, R>(f: F) -> R
    where
        F: FnOnce(&Arc<RuntimeContext>) -> R,
    {
        NioContext::get(|ctx| match ctx {
            NioContext::None => no_rt_found_panic(),
            NioContext::Runtime(ctx) => f(ctx),
            NioContext::Local(ctx) => f(&ctx.runtime_ctx),
        })
    }

    pub fn current() -> Arc<RuntimeContext> {
        RuntimeContext::with(Arc::clone)
    }

    pub fn metrics(self: Arc<Self>) -> metrics::RuntimeMetrics {
        metrics::RuntimeMetrics { ctx: self }
    }

    pub fn enter(self: Arc<Self>) {
        NioContext::enter(self);
    }

    pub fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (task, join) = BlockingTask::spawn(f);
        self.threadpool.execute(task);
        join
    }

    pub fn spawn<F>(self: &Arc<Self>, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (task, join) = Scheduler::spawn(self.clone(), future);
        self.send_task_at(self.workers.least_loaded_worker(), task);
        join
    }

    pub fn spawn_pinned_at<F, Fut>(self: &Arc<Self>, id: u8, future: F) -> JoinHandle<Fut::Output>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future + 'static,
        Fut::Output: Send + 'static,
    {
        let id = self.workers.id(id);
        let (task, join) = unsafe { LocalScheduler::spawn(id, self.clone(), future()) };
        self.send_task_at(id, task);
        join
    }

    pub fn spawn_pinned<F, Fut>(self: &Arc<Self>, future: F) -> JoinHandle<Fut::Output>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future + 'static,
        Fut::Output: Send + 'static,
    {
        let id = self.workers.least_loaded_worker();
        let (task, join) = unsafe { LocalScheduler::spawn(id, self.clone(), future()) };
        self.send_task_at(id, task);
        join
    }

    pub(crate) fn send_task_to_least_loaded_worker(&self, task: Task) {
        self.send_task_at(self.workers.least_loaded_worker(), task);
    }

    pub(crate) fn send_task_at(&self, id: WorkerId, task: Task) {
        self.workers.shared_queue(id).push(task);

        let task_queue = self.workers.task_queue(id);
        let state = task_queue.increase_shared_and_mark_as_notified();
        if !state.is_notified() {
            #[cfg(feature = "metrics")]
            self.measurement.queue_notified(id.get());

            if let Err(_err) = self.workers.notifier(id).wake() {
                task_queue.clear_notified_flag();
                #[cfg(debug_assertions)]
                eprintln!("notifier error: {_err}");
            }
        }
    }
}
