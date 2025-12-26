use super::*;
use local_queue::LocalQueue;
use std::cell::UnsafeCell;

thread_local! {
    static LOCAL_CONTEXT: UnsafeCell<Option<LocalContext>> = const { UnsafeCell::new(None) };
}

pub struct LocalContext {
    pub worker_id: u8,
    pub local_queue: LocalQueue,
    pub runtime_ctx: Arc<RuntimeContext>,
}

impl LocalContext {
    pub fn init(local_queue: LocalQueue, id: u8, runtime_ctx: Arc<RuntimeContext>) {
        LOCAL_CONTEXT.with(|ctx| unsafe {
            *ctx.get() = Some(Self {
                local_queue,
                runtime_ctx,
                worker_id: id,
            })
        });
    }

    pub fn with<F, R>(f: F) -> R
    where
        F: FnOnce(&Self) -> R,
    {
        LOCAL_CONTEXT
            .with(|ctx| unsafe { f((*ctx.get()).as_ref().expect("no `Nio` runtime found")) })
    }

    pub fn current(&self) -> &Worker {
        unsafe {
            self.runtime_ctx
                .workers
                .get_unchecked(self.worker_id as usize)
        }
    }
}
