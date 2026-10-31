use crate::{
    rt::context::LocalContext,
    timer::sleep::{Sleep, sleep},
};
use std::{
    future::poll_fn,
    ops::{Deref, DerefMut},
    task::{Context, Poll},
    time::{Duration, Instant},
};

#[derive(Debug)]
pub struct Interval {
    delay: Sleep,
    period: Duration,
}

impl Interval {
    #[inline]
    pub fn at(deadline: Instant, period: Duration) -> Interval {
        Interval {
            delay: Sleep::at(deadline),
            period,
        }
    }

    pub fn period(&self) -> Duration {
        self.period
    }

    pub fn set_period(&mut self, period: Duration) {
        self.period = period;
    }

    pub fn tick<'a>(&'a mut self) -> impl Future + use<'a> {
        poll_fn(|cx| self.poll(cx))
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        self.delay.timer.as_ref().waker.register(cx);

        if self.delay.is_elapsed() {
            let timer = self.delay.timer.clone();

            LocalContext::with(|ctx| unsafe {
                ctx.timers(|timers| {
                    timer.set_deadline(timers.clock.current() + self.period);
                    timers.insert_entry(timer)
                })
            });

            return Poll::Ready(());
        }
        Poll::Pending
    }
}

pub fn interval(period: Duration) -> Interval {
    Interval {
        delay: sleep(Duration::ZERO),
        period,
    }
}

impl Deref for Interval {
    type Target = Sleep;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.delay
    }
}

impl DerefMut for Interval {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.delay
    }
}
