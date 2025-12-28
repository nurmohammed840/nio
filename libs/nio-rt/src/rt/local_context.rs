use super::*;

use std::{cell::UnsafeCell, collections::VecDeque, rc::Rc};
use task::{JoinHandle, Metadata, Scheduler, Task, TaskKind};
use task_counter::{Counter, TaskCounter};
use worker::{SharedQueue, WorkerId};

thread_local! {
    static LOCAL_CONTEXT: UnsafeCell<Option<Rc<LocalContext>>> = UnsafeCell::new(None);
}

pub struct LocalContext {
    pub worker_id: WorkerId,
    pub local_queue: UnsafeCell<VecDeque<Task>>,
    pub runtime_ctx: Arc<RuntimeContext>,
}

impl LocalContext {
    pub fn new(worker_id: WorkerId, cap: usize, runtime_ctx: Arc<RuntimeContext>) -> Rc<Self> {
        Rc::new(Self {
            worker_id,
            local_queue: UnsafeCell::new(VecDeque::with_capacity(cap)),
            runtime_ctx,
        })
    }

    pub fn init(self: Rc<Self>) {
        LOCAL_CONTEXT.with(|ctx| unsafe { *ctx.get() = Some(self) });
    }

    pub fn with<F, R>(f: F) -> R
    where
        F: FnOnce(&Self) -> R,
    {
        Self::try_with(|ctx| f(ctx.expect("no `Nio` runtime found")))
    }

    pub fn try_with<F, R>(f: F) -> R
    where
        F: FnOnce(Option<&Rc<Self>>) -> R,
    {
        LOCAL_CONTEXT.with(|ctx| unsafe { f((*ctx.get()).as_ref()) })
    }

    // ------------------------------------------------------------------------

    pub fn add_task_to_local_queue(&self, task: Task) {
        unsafe { self.local_queue().push_back(task) };
        let counter = self.task_counter().increase_local();
        unsafe { self.move_tasks_from_shared_to_local_queue(counter) };
    }

    pub fn spawn_local<Fut>(&self, future: Fut) -> JoinHandle<Fut::Output>
    where
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        let (task, join) = Task::new_local_with(
            Metadata {
                kind: TaskKind::Pinned(self.worker_id),
            },
            future,
            Scheduler {
                runtime_ctx: self.runtime_ctx.clone(),
            },
        );
        self.add_task_to_local_queue(task);
        join
    }

    /// Safety: the caller must ensure that there are no `local_queue` references alive.
    pub unsafe fn local_queue(&self) -> &mut VecDeque<Task> {
        unsafe { &mut *self.local_queue.get() }
    }

    /// Safety: the caller must ensure that there are no `local_queue` references alive.
    pub unsafe fn move_tasks_from_shared_to_local_queue(&self, counter: Counter) {
        let count = counter.shared();
        if count > 0 {
            let shared_queue = self.shared_queue();
            for _ in 0..count {
                let task = shared_queue.pop().unwrap();
                unsafe { self.local_queue() }.push_back(task);
            }
            self.task_counter().move_shared_to_local(counter);
        }
    }

    #[inline]
    pub fn task_counter(&self) -> &TaskCounter {
        self.runtime_ctx.workers.task_counter(self.worker_id)
    }

    #[inline]
    pub fn shared_queue(&self) -> &SharedQueue {
        self.runtime_ctx.workers.shared_queue(self.worker_id)
    }
}
