pub use nio::{sleep, spawn, timeout, spawn_pinned};
use std::future::Future;

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

    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        nio_future::block_on(self.0.spawn(future)).unwrap()
    }
}
