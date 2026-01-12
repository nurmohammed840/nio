use nio::{Sleep, test};
use std::time::{Duration, Instant};

mod support {
    pub mod futures;
}
use support::futures::test::{assert_pending, task};

#[test]
async fn immediate_sleep() {
    for _ in 0..1000 {
        let now = Instant::now();
        let mut sleep = Sleep::at(now);
        (&mut sleep).await;
        assert!(sleep.is_elapsed())
    }
}

#[test]
async fn is_elapsed() {
    let now = Instant::now();
    let mut sleep = Sleep::at(now + Duration::from_millis(10));

    assert!(!sleep.is_elapsed());
    assert!(futures::poll!(&mut sleep).is_pending());
    assert!(!sleep.is_elapsed());

    (&mut sleep).await;
    assert!(sleep.is_elapsed());
}

#[test]
async fn reset_future_sleep_before_fire() {
    let now = Instant::now();

    let mut sleep = task::spawn(Sleep::at(now + ms(100)));
    assert_pending!(sleep.poll());

    let mut sleep = sleep.into_inner();

    sleep.reset_at(Instant::now() + ms(200));
    sleep.await;

    assert!(now.elapsed() >= ms(200));
}

#[test]
async fn reset_past_sleep_before_fire() {
    let now = Instant::now();

    let mut sleep = task::spawn(Sleep::at(now + ms(200)));
    assert_pending!(sleep.poll());

    let mut sleep = sleep.into_inner();

    nio::sleep(ms(20)).await;

    sleep.reset_at(now + ms(80));
    sleep.await;

    assert!(now.elapsed() >= ms(80));
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
