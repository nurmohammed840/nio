use super::*;
use task::*;

pub struct RuntimeContext {
    pub(crate) workers: Box<[Worker]>,
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
            |_| {},
        );
        self.workers[0].shared_queue.push(task);
        join
    }

    pub fn spawn_pinned_at<F, Fut>(&self, worker_id: u8, future: F) -> JoinHandle<Fut::Output>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        spawn_pinned_at(&self.workers[worker_id as usize], future)
    }

    pub fn spawn_pinned<F, Fut>(&self, future: F) -> JoinHandle<Fut::Output>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        let worker = unsafe {
            crate::utils::min_by_key(&self.workers, |worker| worker.task_counter.load().total())
        };
        spawn_pinned_at(worker, future)
    }
}

fn spawn_pinned_at<F, Fut>(worker: &Worker, future: F) -> JoinHandle<Fut::Output>
where
    F: FnOnce() -> Fut + Send,
    Fut: Future + 'static,
    Fut::Output: 'static,
{
    let (task, join) = Task::new_local_with(
        Metadata {
            kind: TaskKind::Pinned(worker.id),
        },
        future(),
        |_| {},
    );
    worker.shared_queue.push(task);
    join
}
