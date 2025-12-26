use super::*;
use task::*;
use worker::SharedQueue;

pub struct RuntimeContext {
    pub(crate) workers: Workers,
    pub(crate) threadpool: ThreadPool<BlockingTask>,
}

impl RuntimeContext {
    pub fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (task, join) = BlockingTask::new(f);
        self.threadpool.execute(task);
        join
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (task, join) = Task::new_with(
            Metadata {
                kind: TaskKind::Sendable,
            },
            future,
            Scheduler,
        );
        let index = self.workers.least_loaded_worker_index();
        self.workers.shared_queues[index].push(task);
        self.workers.task_counters[index].increase_shared();
        join
    }

    pub fn spawn_pinned_at<F, Fut>(&self, worker_id: u8, future: F) -> JoinHandle<Fut::Output>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        spawn_pinned_at(worker_id, &self.workers, future)
    }

    pub fn spawn_pinned<F, Fut>(&self, future: F) -> JoinHandle<Fut::Output>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        let idx = self.workers.least_loaded_worker_index();
        spawn_pinned_at(idx as u8, &self.workers, future)
    }
}

fn spawn_pinned_at<F, Fut>(id: u8, workers: &Workers, future: F) -> JoinHandle<Fut::Output>
where
    F: FnOnce() -> Fut + Send,
    Fut: Future + 'static,
    Fut::Output: 'static,
{
    let (task, join) = Task::new_local_with(
        Metadata {
            kind: TaskKind::Pinned(id),
        },
        future(),
        Scheduler,
    );
    let index = id as usize;
    workers.shared_queues[index].push(task);
    workers.task_counters[index].increase_shared();
    join
}
