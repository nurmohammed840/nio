#![allow(unused)]

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use nio_task::Status;

pub struct Runnable<M = ()> {
    task: nio_task::Task<M>,
}

impl Runnable {
    pub fn run(self) -> bool {
        match self.task.poll() {
            Status::Pending | Status::Complete(_) => false,
            Status::Yielded(task) => {
                task.schedule();
                true
            }
        }
    }

    pub fn schedule(self) {
        self.task.schedule();
    }
}

pub struct Task<T> {
    join: Option<nio_task::JoinHandle<T>>,
}

impl<T> Future for Task<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let join = unsafe { self.map_unchecked_mut(|a| a.join.as_mut().unwrap()) };
        match join.poll(cx) {
            Poll::Ready(t) => Poll::Ready(t.expect("Task polled after completion")),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        if let Some(task) = self.join.take() {
            task.abort();
        }
    }
}

impl<T> Task<T> {
    pub fn detach(mut self) {
        self.join.take();
    }

    pub async fn cancel(mut self) -> Option<T> {
        let join = self.join.take().unwrap();
        join.abort();
        join.await.ok()
    }
}

pub fn spawn<F, S>(future: F, scheduler: S) -> (Runnable, Task<F::Output>)
where
    F: Future + Send + 'static,
    F::Output: Send,
    S: Schedule + Send + 'static,
{
    let (task, join) = nio_task::Task::new(future, move |task| {
        scheduler.schedule(Runnable { task });
    });
    (Runnable { task }, Task { join: Some(join) })
}

pub trait Schedule<M = ()> {
    fn schedule(&self, runnable: Runnable<M>);
}

impl<M, F> Schedule<M> for F
where
    F: Fn(Runnable<M>),
{
    fn schedule(&self, runnable: Runnable<M>) {
        self(runnable)
    }
}
