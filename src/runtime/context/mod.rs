// mod local_queue;

use crate::{blocking::ThreadPool, io::ReactorContext, scheduler::Scheduler};
use std::{cell::UnsafeCell, sync::Arc};

// pub use local_queue::LocalQueue;

thread_local! {
    static CONTEXT: UnsafeCell<Option<Arc<RuntimeContext>>> = const { UnsafeCell::new(None) };
    // static LOCAL_QUEUE: UnsafeCell<Option<LocalQueue>> = const { UnsafeCell::new(None) };
}

pub struct RuntimeContext {
    pub scheduler: Scheduler,
    pub thread_pool: ThreadPool,
    pub reactor_ctx: Option<ReactorContext>,
}

impl RuntimeContext {
    pub fn init(self: Arc<Self>) {
        CONTEXT.with(|ctx| unsafe { *ctx.get() = Some(self) });
    }

    pub fn reactor(&self) -> &ReactorContext {
        self.reactor_ctx.as_ref().expect("no reactor found")
    }
}

pub fn with<F, R>(f: F) -> R
where
    F: FnOnce(&Arc<RuntimeContext>) -> R,
{
    CONTEXT.with(|ctx| unsafe { f((*ctx.get()).as_ref().expect("no `Nio` runtime found")) })
}
