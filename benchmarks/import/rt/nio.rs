pub use nio::{sleep, spawn, spawn_blocking, spawn_pinned, timeout, spawn_local};
use std::{future::Future, sync::Arc};

pub struct Runtime(nio::Runtime);

impl Runtime {
    pub fn new(core: usize) -> Runtime {
        Self(
            nio::RuntimeBuilder::new()
                .worker_threads(core as u8)
                .rt()
                .unwrap(),
        )
    }

    pub fn multi() -> Runtime {
        Self(
            nio::RuntimeBuilder::new()
                .max_blocking_threads(512)
                .rt()
                .unwrap(),
        )
    }

    pub fn spawn<F>(&self, future: F) -> nio::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.0.spawn(future)
    }

    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        nio_future::block_on(self.0.spawn_pinned_at(0, || future)).unwrap()
    }
}
