use std::{
    future::pending,
    time::{Duration, Instant},
};

use nio::{Sleep, Timeout, spawn_local, spawn_pinned, test};

const ITER: usize = if cfg!(miri) { 20 } else { 500 };
const DELAY: Duration = Duration::from_millis(10);

async fn sleep_short() {
    let now = Instant::now();
    Sleep::at(now + DELAY).await;

    let elapsed = now.elapsed();
    assert!(elapsed >= DELAY, "{elapsed:?}");
}

async fn timeout_expired() {
    let now = Instant::now();
    let timer = Timeout::at(now + DELAY, pending::<()>());
    assert!(timer.await.is_none());

    let elapsed = now.elapsed();
    assert!(elapsed >= DELAY, "{elapsed:?}");
}

async fn timeout_short() {
    let now = Instant::now();
    let timer = Timeout::at(now + Duration::from_secs(60), Sleep::at(now + DELAY));
    assert!(timer.await.is_some());

    let elapsed = now.elapsed();
    assert!(elapsed >= DELAY, "{elapsed:?}");
}

#[test(worker_threads = 2)]
async fn sleep() {
    let mut tasks = Vec::with_capacity(ITER * 2);
    for _ in 0..ITER {
        tasks.push(spawn_local(sleep_short()));
        tasks.push(spawn_pinned(sleep_short));
    }

    for task in tasks {
        task.await.unwrap();
    }
}

#[test(worker_threads = 2)]
async fn timeout() {
    let mut tasks = Vec::with_capacity(ITER * 4);
    for _ in 0..ITER {
        tasks.push(spawn_local(timeout_short()));
        tasks.push(spawn_local(timeout_expired()));

        tasks.push(spawn_pinned(timeout_short));
        tasks.push(spawn_pinned(timeout_expired));
    }

    for task in tasks {
        task.await.unwrap();
    }
}

#[test(worker_threads = 2)]
async fn starving() {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    struct Starve<T: Future<Output = ()>>(T);
    impl<T> Starve<T>
    where
        T: Future<Output = ()>,
    {
        fn new(fut: T) -> Starve<Pin<Box<T>>> {
            Starve(Box::pin(fut))
        }
    }

    impl<T: Future<Output = ()> + Unpin> Future for Starve<T> {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            if Pin::new(&mut self.0).poll(cx).is_ready() {
                return Poll::Ready(());
            }
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }

    let mut tasks = Vec::with_capacity(ITER * 6);
    for _ in 0..ITER {
        tasks.push(spawn_local(Starve::new(sleep_short())));
        tasks.push(spawn_pinned(|| Starve::new(sleep_short())));

        tasks.push(spawn_local(Starve::new(timeout_short())));
        tasks.push(spawn_pinned(|| Starve::new(timeout_short())));

        tasks.push(spawn_local(Starve::new(timeout_expired())));
        tasks.push(spawn_pinned(|| Starve::new(timeout_expired())));
    }

    for task in tasks {
        task.await.unwrap();
    }
}
