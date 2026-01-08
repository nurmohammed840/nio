use std::{
    future::{Future, poll_fn},
    pin::Pin,
    time::Duration,
};

use futures_lite::FutureExt;
use nio_rt::{Interval, sleep, spawn_local};

struct Timer(pub std::time::Instant);

impl Timer {
    fn now() -> Self {
        Self(std::time::Instant::now())
    }

    fn elapsed(&self) -> Duration {
        self.0.elapsed() + Duration::from_millis(50)
    }
}

#[nio_rt::test]
async fn smoke() {
    let start = Timer::now();
    sleep(Duration::from_secs(1)).await;

    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_secs(1));
}

#[nio_rt::test]
async fn interval() {
    let period = Duration::from_secs(1);
    let jitter = Duration::from_millis(500);
    let start = Timer::now();
    let mut timer = Interval::at(start.0 + period, period);

    timer.tick().await;

    let elapsed = start.elapsed();
    assert!(elapsed >= period);
    assert!(elapsed - period < jitter);

    timer.tick().await;

    let elapsed = start.elapsed();
    assert!(elapsed >= period * 2);
    assert!(elapsed - period * 2 < jitter);
}

#[nio_rt::test]
async fn poll_across_tasks() {
    let start = Timer::now();
    let (sender, mut receiver) = tokio::sync::mpsc::channel(1);

    let task1 = spawn_local(async move {
        let mut timer = sleep(Duration::from_secs(1));

        async {
            (&mut timer).await;
            panic!("timer should not be ready")
        }
        .or(async {})
        .await;

        sender.send(timer).await.ok();
    });

    let task2 = spawn_local(async move {
        let timer = receiver.recv().await.unwrap();
        timer.await;
    });

    task1.await.unwrap();
    task2.await.unwrap();

    assert!(start.elapsed() >= Duration::from_secs(1));
}

#[nio_rt::test]
async fn set() {
    let start = Timer::now();
    let mut timer = sleep(Duration::from_secs(5));

    poll_fn(|cx| {
        let _ = Pin::new(&mut timer).poll(cx);
        std::task::Poll::Ready(())
    })
    .await;

    timer.reset(Duration::from_secs(1));

    (&mut timer).await;

    assert!(timer.is_elapsed());
    assert!(start.elapsed() >= Duration::from_secs(1));
    assert!(start.elapsed() < Duration::from_secs(5));
}
