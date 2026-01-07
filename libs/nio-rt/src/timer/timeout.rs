use crate::timer::sleep::{Sleep, sleep};
use std::{
    fmt,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

pub struct Timeout<Fut> {
    delay: Sleep,
    fut: Fut,
}

impl<F> Unpin for Timeout<F> {}

impl<T> Timeout<T> {
    pub fn at<F>(deadline: Instant, future: F) -> Timeout<F::IntoFuture>
    where
        F: IntoFuture,
    {
        Timeout {
            delay: Sleep::at(deadline),
            fut: future.into_future(),
        }
    }

    #[inline]
    pub fn get_ref(&self) -> &T {
        &self.fut
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.fut
    }

    pub fn into_inner(self) -> T {
        self.fut
    }
}

pub fn timeout<F>(duration: Duration, future: F) -> Timeout<F::IntoFuture>
where
    F: IntoFuture,
{
    Timeout {
        delay: sleep(duration),
        fut: future.into_future(),
    }
}

impl<Fut: Future> Future for Timeout<Fut> {
    type Output = Option<Fut::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        match unsafe { Pin::new_unchecked(&mut this.fut).poll(cx) } {
            Poll::Ready(val) => Poll::Ready(Some(val)),
            Poll::Pending => {
                if this.delay.is_elapsed() {
                    return Poll::Ready(None);
                }
                this.delay.timer.waker.register(cx);
                Poll::Pending
            }
        }
    }
}

impl<T> Deref for Timeout<T> {
    type Target = Sleep;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.delay
    }
}

impl<T> DerefMut for Timeout<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.delay
    }
}

impl<T> fmt::Debug for Timeout<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Timeout").field(&self.delay).finish()
    }
}
