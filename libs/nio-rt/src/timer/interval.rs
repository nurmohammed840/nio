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

    pub async fn tick(&mut self) {
        poll_fn(|cx| self.poll(cx)).await
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        self.delay.timer.waker.register(cx);

        if self.delay.is_elapsed() {
            let timer = self.delay.timer.clone();
            LocalContext::with(|ctx| unsafe {
                ctx.timers(|timers| {
                    timer.notified.set(false);
                    timer.deadline.set(timers.clock.current() + self.period);
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

#[cfg(test)]
mod tests {
    #[allow(warnings)]
    use crate::timer::sleep::sleep;

    use super::*;

    #[crate::test(crate)]
    async fn test_sname() {
        let mut interval = interval(Duration::from_secs(1));

        for _ in 0..5 {
            interval.tick().await;
            // sleep(Duration::from_secs(1)).await;
            println!("sad")
        }
    }
}
