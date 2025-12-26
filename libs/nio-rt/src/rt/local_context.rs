use super::*;
use std::{cell::UnsafeCell, collections::VecDeque, rc::Rc};
use task::Task;
use task_counter::Counter;

thread_local! {
    static LOCAL_CONTEXT: UnsafeCell<Option<Rc<LocalContext>>> = UnsafeCell::new(None);
}

pub struct LocalContext {
    worker_id: u8,
    local_queue: UnsafeCell<VecDeque<Task>>,
    runtime_ctx: Arc<RuntimeContext>,
}

impl LocalContext {
    pub fn new(worker_id: u8, cap: usize, runtime_ctx: Arc<RuntimeContext>) -> Rc<Self> {
        Rc::new(Self {
            worker_id,
            local_queue: UnsafeCell::new(VecDeque::with_capacity(cap)),
            runtime_ctx,
        })
    }

    pub fn init(self: Rc<Self>) {
        LOCAL_CONTEXT.with(|ctx| unsafe { *ctx.get() = Some(self) });
    }

    /// Safety: the caller must ensure that there are no `local_queue` references alive.
    pub unsafe fn local_queue(&self) -> &mut VecDeque<Task> {
        unsafe { &mut *self.local_queue.get() }
    }

    pub fn with<F, R>(f: F) -> R
    where
        F: FnOnce(&Self) -> R,
    {
        LOCAL_CONTEXT
            .with(|ctx| unsafe { f((*ctx.get()).as_ref().expect("no `Nio` runtime found")) })
    }

    /// Safety: the caller must ensure that there are no `local_queue` references alive.
    pub unsafe fn move_tasks_from_shared_to_local_queue(&self, counter: Counter) {
        let count = counter.shared();
        if count > 0 {
            let worker = self.current();
            for _ in 0..count {
                let task = worker.shared_queue.pop().unwrap();
                unsafe { self.local_queue() }.push_back(task);
            }
            worker.task_counter.move_shared_to_local(counter);
        }
    }

    pub fn current(&self) -> &Worker {
        unsafe {
            self.runtime_ctx
                .workers
                .get_unchecked(self.worker_id as usize)
        }
    }
}
