use std::{
    future::{Future, poll_fn},
    pin::Pin,
    time::{Duration, Instant},
};

use futures_lite::FutureExt;
use nio_rt::{Interval, sleep, spawn_local};

#[nio_rt::test]
async fn smoke() {
    let start = Instant::now();
    sleep(Duration::from_secs(1)).await;
    assert!(start.elapsed() >= Duration::from_secs(1));
}

#[nio_rt::test]
async fn interval() {
    let period = Duration::from_secs(1);
    let jitter = Duration::from_millis(500);
    let start = Instant::now();
    let mut timer = Interval::at(start + period, period);
    timer.tick().await;
    let elapsed = start.elapsed();
    assert!(elapsed >= period && elapsed - period < jitter);
    timer.tick().await;
    let elapsed = start.elapsed();
    assert!(elapsed >= period * 2 && elapsed - period * 2 < jitter);
}

#[nio_rt::test]
async fn poll_across_tasks() {
    let start = Instant::now();
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
    let start = Instant::now();
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
