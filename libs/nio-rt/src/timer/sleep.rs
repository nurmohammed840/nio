use crate::rt::context::LocalContext;

use super::*;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub struct Sleep {
    pub(crate) timer: Rc<Timer>,
}

impl Sleep {
    #[inline]
    pub fn deadline(&self) -> Instant {
        self.timer.deadline.get()
    }

    #[inline]
    pub fn is_elapsed(&self) -> bool {
        self.timer.state.get() == State::Notified
    }

    pub fn reset(&mut self, deadline: Instant) {
        LocalContext::with(|ctx| unsafe {
            ctx.timers(|timers| timers.reset(&self.timer, deadline))
        })
    }
}

pub fn sleep(duration: Duration) -> Sleep {
    LocalContext::with(|ctx| unsafe { ctx.timers(|timers| timers.sleep(duration)) })
}

impl Future for Sleep {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.is_elapsed() {
            return Poll::Ready(());
        }
        self.timer.waker.register(cx);
        Poll::Pending
    }
}

impl Drop for Sleep {
    fn drop(&mut self) {
        LocalContext::with(|ctx| unsafe { ctx.timers(|timers| timers.remove(&self.timer)) })
    }
}
