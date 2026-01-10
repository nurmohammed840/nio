#![warn(rust_2018_idioms)]

use nio_future::yield_now;
use nio_rt::{spawn, spawn_local};

use std::thread;
use std::time::Duration;

mod support {
    pub mod futures;
    pub(crate) mod mpsc_stream;
}

use support::futures::sync::oneshot;
use support::futures::test::assert_ok;

#[test]
fn no_extra_poll() {
    use futures_lite::{Stream, StreamExt};
    use pin_project_lite::pin_project;
    use std::pin::Pin;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering::SeqCst},
    };
    use std::task::{Context, Poll};

    pin_project! {
        struct TrackPolls<S> {
            npolls: Arc<AtomicUsize>,
            #[pin]
            s: S,
        }
    }

    impl<S> Stream for TrackPolls<S>
    where
        S: Stream,
    {
        type Item = S::Item;
        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let this = self.project();
            this.npolls.fetch_add(1, SeqCst);
            this.s.poll_next(cx)
        }
    }

    let (tx, rx) = support::mpsc_stream::unbounded_channel_stream::<()>();
    let rx = TrackPolls {
        npolls: Arc::new(AtomicUsize::new(0)),
        s: rx,
    };
    let npolls = Arc::clone(&rx.npolls);

    let rt = rt(1);

    // TODO: could probably avoid this, but why not.
    let mut rx = Box::pin(rx);

    rt.block_on(|| async move {
        spawn_local(async move { while rx.next().await.is_some() {} });
        yield_now().await;

        // should have been polled exactly once: the initial poll
        assert_eq!(npolls.load(SeqCst), 1);

        tx.send(()).unwrap();
        yield_now().await;

        // should have been polled twice more: once to yield Some(), then once to yield Pending
        assert_eq!(npolls.load(SeqCst), 1 + 2);
        drop(tx);

        yield_now().await;

        // should have been polled once more: to yield None
        assert_eq!(npolls.load(SeqCst), 1 + 2 + 1);
    });
}

#[test]
fn drop_future_if_no_waker_exist() {
    use std::future::pending;

    let (tx1, rx1) = oneshot::channel();

    rt(1).block_on(|| async {
        // Spawn a task that will never notify
        spawn_local(async move {
            pending::<()>().await;
            // Rt drop this future, Bcs it will never wake up.
            tx1.send(()).unwrap();
        });

        yield_now().await;
        assert!(rx1.await.is_err());
    });
}

#[test]
fn spawn_two() {
    let rt = rt(2);
    let out = rt.block_on(|| async {
        let (tx, rx) = oneshot::channel();
        spawn(async move {
            spawn(async move {
                tx.send("ZOMG").unwrap();
            });
        });
        assert_ok!(rx.await)
    });
    assert_eq!(out, "ZOMG");
}

#[cfg_attr(target_os = "wasi", ignore = "WASI: std::thread::spawn not supported")]
#[test]
fn spawn_remote() {
    let rt = rt(2);
    let out = rt.block_on(|| async {
        let (tx, rx) = oneshot::channel();
        let handle = spawn(async move {
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(10));
                tx.send("ZOMG").unwrap();
            });
            rx.await.unwrap()
        });
        handle.await.unwrap()
    });
    assert_eq!(out, "ZOMG");
}

fn rt(core: u8) -> nio_rt::Runtime {
    nio_rt::RuntimeBuilder::new()
        .worker_threads(core)
        .rt()
        .unwrap()
}
