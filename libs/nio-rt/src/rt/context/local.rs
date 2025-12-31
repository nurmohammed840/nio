use crate::timer::Timers;

use super::*;
use task::*;

use std::{cell::UnsafeCell, collections::VecDeque, rc::Rc};
use task_counter::{Counter, TaskCounter};
use worker::{SharedQueue, WorkerId};

pub struct LocalContext {
    timers: UnsafeCell<Timers>,
    local_queue: UnsafeCell<VecDeque<Task>>,

    pub worker_id: WorkerId,
    pub runtime_ctx: Arc<RuntimeContext>,
}

impl LocalContext {
    pub fn init(self: Rc<Self>) {
        Context::init(self);
    }

    pub fn with<F, R>(f: F) -> R
    where
        F: FnOnce(&Rc<LocalContext>) -> R,
    {
        Context::get(|ctx| match ctx {
            Context::None => panic!("no `Nio` runtime available"),
            Context::Global(_) => panic!("no `Nio` local runtime found"),
            Context::Local(ctx) => f(ctx),
        })
    }

    pub fn new(worker_id: WorkerId, cap: usize, runtime_ctx: Arc<RuntimeContext>) -> Rc<Self> {
        Rc::new(Self {
            worker_id,
            timers: UnsafeCell::new(Timers::new()),
            local_queue: UnsafeCell::new(VecDeque::with_capacity(cap)),
            runtime_ctx,
        })
    }

    pub fn add_task_to_local_queue(&self, task: Task) {
        unsafe { self.local_queue(|q| q.push_back(task)) };
        let counter = self.task_counter().increase_local();
        self.move_tasks_from_shared_to_local_queue(counter)
    }

    pub fn spawn_local<Fut>(&self, future: Fut) -> JoinHandle<Fut::Output>
    where
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        let (task, join) = LocalScheduler::spawn(self.worker_id, self.runtime_ctx.clone(), future);
        self.add_task_to_local_queue(task);
        join
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
    /// ```rust,ignore
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
    pub unsafe fn local_queue<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut VecDeque<Task>) -> R,
    {
        f(unsafe { &mut *self.local_queue.get() })
    }
  
    /// ## See: safety docs [`LocalContext::local_queue`]
    pub unsafe fn timers<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Timers) -> R,
    {
        f(unsafe { &mut *self.timers.get() })
    }

    /// Safety: the caller must ensure not to call this funtion in [`LocalContext::local_queue`] closure.
    pub fn move_tasks_from_shared_to_local_queue(&self, counter: Counter) {
        let count = counter.shared();
        if count > 0 {
            let shared_queue = self.shared_queue();
            for _ in 0..count {
                let task = shared_queue.pop().unwrap();
                unsafe { self.local_queue(|q| q.push_back(task)) };
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
