use super::{
    error::TimeoutError,
    timer::{sleep, Sleep},
};
use std::{
    future::{Future, IntoFuture},
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Timeout<T> {
    fut: T,
    delay: Sleep,
}

impl<T> Timeout<T> {
    /// Gets a reference to the underlying value in this timeout.
    pub fn get_ref(&self) -> &T {
        &self.fut
    }

    /// Gets a mutable reference to the underlying value in this timeout.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.fut
    }

    /// Consumes this timeout, returning the underlying value.
    pub fn into_inner(self) -> T {
        self.fut
    }
}

pub fn timeout<F>(duration: Duration, future: F) -> Timeout<F::IntoFuture>
where
    F: IntoFuture,
{
    Timeout {
        fut: future.into_future(),
        delay: sleep(duration),
    }
}

impl<F: Future> Future for Timeout<F> {
    type Output = Result<F::Output, TimeoutError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        unsafe {
            let timeout = self.get_unchecked_mut();
            if let Poll::Ready(output) = Pin::new_unchecked(&mut timeout.fut).poll(cx) {
                return Poll::Ready(Ok(output));
            }
            Pin::new(&mut timeout.delay)
                .poll(cx)
                .map(|()| Err(TimeoutError::new()))
        }
    }
}
