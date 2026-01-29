use crate::timer::Timers;

use super::*;
use task::*;

use std::{cell::UnsafeCell, collections::VecDeque, rc::Rc};
use task_queue::{Counter, TaskQueue};
use worker::{SharedQueue, WorkerId};

pub struct LocalContext {
    timers: UnsafeCell<Timers>,
    local_queue: UnsafeCell<VecDeque<Task>>,

    pub(crate) worker_id: WorkerId,
    pub(crate) runtime_ctx: Arc<RuntimeContext>,
    pub(crate) io_registry: driver::Registry,
}

impl LocalContext {
    pub(crate) fn new(
        worker_id: WorkerId,
        cap: usize,
        runtime_ctx: Arc<RuntimeContext>,
        io_registry: driver::Registry,
    ) -> Rc<Self> {
        LocalContext {
            worker_id,
            timers: UnsafeCell::new(Timers::new()),
            local_queue: UnsafeCell::new(VecDeque::with_capacity(cap)),
            runtime_ctx,
            io_registry,
        }
        .into()
    }

    pub(crate) fn init(self: Rc<Self>) {
        NioContext::init(self);
    }

    pub(crate) fn with<F, R>(f: F) -> R
    where
        F: FnOnce(&Rc<LocalContext>) -> R,
    {
        NioContext::get(|ctx| match ctx {
            NioContext::None => no_rt_found_panic(),
            NioContext::Runtime(_) => no_local_rt_found_panic(),
            NioContext::Local(ctx) => f(ctx),
        })
    }

    pub fn current() -> Rc<LocalContext> {
        LocalContext::with(Rc::clone)
    }

    pub fn worker_id(&self) -> WorkerId {
        self.worker_id
    }

    pub fn spawn_local<Fut>(&self, future: Fut) -> JoinHandle<Fut::Output>
    where
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        let (task, join) =
            unsafe { LocalScheduler::spawn(self.worker_id, self.runtime_ctx.clone(), future) };

        self.add_task_to_local_queue(task);
        join
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (task, join) = Scheduler::spawn(self.runtime_ctx.clone(), future);
        let id = self.runtime_ctx.workers.least_loaded_worker();

        if self.worker_id == id {
            self.add_task_to_local_queue(task);
        } else {
            self.runtime_ctx.send_task_at(id, task);
        }
        join
    }

    pub fn spawn_pinned<F, Fut>(&self, future: F) -> JoinHandle<Fut::Output>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future + 'static,
        Fut::Output: Send + 'static,
    {
        let id = self.runtime_ctx.workers.least_loaded_worker();
        let (task, join) = unsafe { LocalScheduler::spawn(id, self.runtime_ctx.clone(), future()) };

        if self.worker_id == id {
            self.add_task_to_local_queue(task);
        } else {
            self.runtime_ctx.send_task_at(id, task);
        }
        join
    }

    pub(crate) fn add_task_to_local_queue(&self, task: Task) {
        unsafe { self.local_queue(|q| q.push_back(task)) };
        let counter = self.task_queue().increase_local();
        self.move_tasks_from_shared_to_local_queue(counter)
    }

    /// # Safety
    /// The caller must ensure that there are no `local_queue` references alive.
    ///
    /// In particular, `local_queue(..)` must **not** be called:
    /// - Recursively
    /// - Indirectly via another function while a queue reference is active.
    ///
    /// Violating this rule may results in **undefined behavior**.
    ///
    /// For example, calling [`LocalContext::local_queue`] recursively or within closures
    /// can lead to undefined behavior.
    ///
    /// Caller can't do this:
    ///
    /// ```ignore
    /// ctx.local_queue(|q1| {
    ///     ctx.local_queue(|q2| {
    ///         // ❌ Undefined behavior!
    ///     });
    /// });
    ///
    /// fn other() {
    ///     LocalContext::local(|ctx| {
    ///         ctx.local_queue(|q2| { ... });
    ///     });
    /// }
    /// ctx.local_queue(|q1| {
    ///    other(); // ❌ Undefined behavior!
    /// });
    /// ```
    ///
    /// ## Note
    ///
    /// To uphold safety, Called should not call any other function.
    ///
    /// Caller must **only mutate** [`VecDeque`]
    pub(crate) unsafe fn local_queue<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut VecDeque<Task>) -> R,
    {
        f(unsafe { &mut *self.local_queue.get() })
    }

    /// ## See: safety docs [`LocalContext::local_queue`]
    pub(crate) unsafe fn timers<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Timers) -> R,
    {
        f(unsafe { &mut *self.timers.get() })
    }

    /// Safety: the caller must ensure not to call this funtion in [`LocalContext::local_queue`] closure.
    pub(crate) fn move_tasks_from_shared_to_local_queue(&self, counter: Counter) {
        let count = counter.shared();
        if count > 0 {
            let shared_queue = self.shared_queue();
            for _ in 0..count {
                let task = shared_queue.pop().unwrap();
                unsafe { self.local_queue(|q| q.push_back(task)) };
            }
            self.task_queue().move_shared_to_local(counter);
        }
    }

    #[inline]
    pub(crate) fn task_queue(&self) -> &TaskQueue {
        self.runtime_ctx.workers.task_queue(self.worker_id)
    }

    #[inline]
    pub(crate) fn shared_queue(&self) -> &SharedQueue {
        self.runtime_ctx.workers.shared_queue(self.worker_id)
    }
}
