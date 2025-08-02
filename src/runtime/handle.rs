use super::{
    context::{self, RuntimeContext},
    task::JoinHandle,
};
use std::{future::Future, marker::PhantomData, mem, sync::Arc};

/// Handle to the runtime.
///
/// The handle is internally reference-counted and can be freely cloned. A handle can be
/// obtained using the [`Runtime::handle`] method.
///
/// [`Runtime::handle`]: crate::runtime::Runtime::handle()
#[derive(Clone)]
pub struct Handle {
    pub(crate) ctx: Arc<RuntimeContext>,
}

#[derive(Debug)]
#[must_use = "Creating and dropping a guard does nothing"]
pub struct EnterGuard<'a> {
    _handle_lt: PhantomData<&'a Handle>,
}

impl EnterGuard<'_> {
    pub(crate) fn new() -> Self {
        Self {
            _handle_lt: PhantomData,
        }
    }
}

impl Handle {
    pub fn current() -> Self {
        Self {
            ctx: context::with(Arc::clone),
        }
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        if mem::size_of::<F>() > crate::BOX_FUTURE_THRESHOLD {
            crate::task::spawn_task(Box::pin(future), &self.ctx)
        } else {
            crate::task::spawn_task(future, &self.ctx)
        }
    }

    pub fn block_on<Fut>(&self, fut: Fut) -> Fut::Output
    where
        Fut: Future,
    {
        crate::future::block_on(fut)
    }

    pub fn enter(&self) -> EnterGuard<'_> {
        self.ctx.clone().init();
        EnterGuard::new()
    }
}
