#![allow(unused)]

use std::future::Future;
pub use tokio::{
    task::{spawn, spawn_blocking},
    time::{sleep, timeout},
};

pub struct Runtime(pub tokio::runtime::Runtime);

impl Runtime {
    pub fn new(workers: usize) -> Runtime {
        let rt = if workers == 1 {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
        } else {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .worker_threads(workers)
                .build()
                .unwrap()
        };
        Self(rt)
    }

    pub fn timer_rt(core: usize) -> Runtime {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(core)
            .build()
            .unwrap();

        Runtime(runtime)
    }

    pub fn multi() -> Runtime {
        Self(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .max_blocking_threads(512)
                .build()
                .unwrap(),
        )
    }

    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.0.spawn(future)
    }

    pub fn handle(&self) -> &tokio::runtime::Handle {
        self.0.handle()
    }

    pub fn print_measurement(&self) {}

    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.0.block_on(future)
    }
}

pub fn spawn_pinned<F, Fut>(future: F) -> tokio::task::JoinHandle<Fut::Output>
where
    F: FnOnce() -> Fut + Send,
    Fut: Future + 'static + Send,
    Fut::Output: Send + 'static,
{
    tokio::spawn(future())
}
