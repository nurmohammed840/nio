use super::*;

pub struct RuntimeContext {
    pub(crate) workers: Workers,
    pub(crate) threadpool: ThreadPool<BlockingTask>,
}

impl RuntimeContext {
    pub fn with<F, R>(f: F) -> R
    where
        F: FnOnce(&Arc<RuntimeContext>) -> R,
    {
        Context::get(|ctx| match ctx {
            Context::None => panic!("no `Nio` runtime available"),
            Context::Global(ctx) => f(ctx),
            Context::Local(ctx) => f(&ctx.runtime_ctx),
        })
    }

    pub fn current() -> Arc<RuntimeContext> {
        RuntimeContext::with(Arc::clone)
    }

    pub fn enter(self: Arc<Self>) {
        Context::enter(self);
    }

    pub fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (task, join) = BlockingTask::new(f);
        self.threadpool.execute(task);
        join
    }

    pub fn spawn<F>(self: &Arc<Self>, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (task, join) = Task::new_with(
            Metadata {
                kind: TaskKind::Sendable,
            },
            future,
            Scheduler {
                runtime_ctx: self.clone(),
            },
        );
        let id = self.workers.least_loaded_worker_id();
        self.workers.shared_queue(id).push(task);
        self.workers.task_counter(id).increase_shared();
        join
    }

    pub fn spawn_pinned_at<F, Fut>(self: &Arc<Self>, id: u8, future: F) -> JoinHandle<Fut::Output>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future + 'static,
        Fut::Output: Send + 'static,
    {
        self._spawn_pinned_at(self.workers.id(id), future)
    }

    pub fn spawn_pinned<F, Fut>(self: &Arc<Self>, future: F) -> JoinHandle<Fut::Output>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future + 'static,
        Fut::Output: Send + 'static,
    {
        let id = self.workers.least_loaded_worker_id();
        self._spawn_pinned_at(id, future)
    }

    fn _spawn_pinned_at<F, Fut>(self: &Arc<Self>, id: WorkerId, fut: F) -> JoinHandle<Fut::Output>
    where
        F: FnOnce() -> Fut,
        Fut: Future + 'static,
    {
        let (task, join) = Task::new_local_with(
            Metadata {
                kind: TaskKind::Pinned(id),
            },
            fut(),
            Scheduler {
                runtime_ctx: self.clone(),
            },
        );
        self.workers.shared_queue(id).push(task);
        self.workers.task_counter(id).increase_shared();
        join
    }

    pub(crate) fn send_task_to_least_loaded_worker(&self, task: Task) {}
    pub(crate) fn send_task_at(&self, id: WorkerId, task: Task) {}
}
