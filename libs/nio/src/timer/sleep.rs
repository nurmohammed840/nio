use crate::rt::context::LocalContext;

use super::*;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub struct Sleep {
    pub(crate) timer: RcTimer,
}

impl Sleep {
    pub fn at(deadline: Instant) -> Sleep {
        LocalContext::with(|ctx| unsafe { ctx.timers(|timers| timers.sleep_at(deadline)) })
    }

    #[inline]
    pub fn deadline(&self) -> Instant {
        self.timer.deadline()
    }

    #[inline]
    pub fn is_elapsed(&self) -> bool {
        self.timer.as_ref().rc.get() == 1
    }

    pub fn reset_at(&mut self, deadline: Instant) {
        LocalContext::with(|ctx| unsafe {
            ctx.timers(|timers| timers.reset_at(&self.timer, deadline))
        })
    }

    pub fn reset(&mut self, duration: Duration) {
        LocalContext::with(|ctx| unsafe {
            ctx.timers(|timers| timers.reset_at(&self.timer, timers.clock.current() + duration))
        })
    }
}

#[inline]
#[must_use]
pub fn sleep(duration: Duration) -> Sleep {
    LocalContext::with(|ctx| unsafe {
        ctx.timers(|timers| timers.sleep_at(timers.clock.current() + duration))
    })
}

impl Future for Sleep {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.is_elapsed() {
            return Poll::Ready(());
        }
        self.timer.as_ref().waker.register(cx);
        Poll::Pending
    }
}

impl Drop for Sleep {
    fn drop(&mut self) {
        if self.is_elapsed() {
            return; // already droped
        }
        LocalContext::with(|ctx| unsafe { ctx.timers(|timers| timers.remove(&self.timer)) })
    }
}

impl fmt::Debug for Sleep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.timer.as_ref(), f)
    }
}
