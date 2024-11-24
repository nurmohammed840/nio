#![allow(unknown_lints, unexpected_cfgs)]
#![warn(rust_2018_idioms)]
#![cfg(all(feature = "full", not(target_os = "wasi")))]
// Too slow on miri.
#![cfg(not(miri))]

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{self, oneshot};

use nio::net::{TcpListener, TcpStream};
use nio::runtime;
use tokio_test::{assert_err, assert_ok};

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{mpsc, Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

macro_rules! cfg_metrics {
    ($($t:tt)*) => {
        #[cfg(all(tokio_unstable, target_has_atomic = "64"))]
        {
            $( $t )*
        }
    }
}

#[test]
fn single_thread() {
    // No panic when starting a runtime w/ a single thread
    let _ = runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .unwrap();
}

#[test]
fn many_oneshot_futures() {
    // used for notifying the main thread
    const NUM: usize = 1_000;

    for _ in 0..5 {
        let (tx, rx) = mpsc::channel();

        let rt = rt();
        let cnt = Arc::new(AtomicUsize::new(0));

        for _ in 0..NUM {
            let cnt = cnt.clone();
            let tx = tx.clone();

            rt.spawn(async move {
                let num = cnt.fetch_add(1, Relaxed) + 1;

                if num == NUM {
                    tx.send(()).unwrap();
                }
            });
        }

        rx.recv().unwrap();

        // Wait for the pool to shutdown
        drop(rt);
    }
}

#[test]
fn spawn_two() {
    let rt = rt();

    let out = rt.block_on(async {
        let (tx, rx) = oneshot::channel();

        nio::spawn(async move {
            nio::spawn(async move {
                tx.send("ZOMG").unwrap();
            });
        });

        assert_ok!(rx.await)
    });

    assert_eq!(out, "ZOMG");

    cfg_metrics! {
        let metrics = rt.metrics();
        drop(rt);
        assert_eq!(1, metrics.remote_schedule_count());

        let mut local = 0;
        for i in 0..metrics.num_workers() {
            local += metrics.worker_local_schedule_count(i);
        }

        assert_eq!(1, local);
    }
}

#[test]
fn many_multishot_futures() {
    const CHAIN: usize = 200;
    const CYCLES: usize = 5;
    const TRACKS: usize = 50;

    for _ in 0..50 {
        let rt = rt();
        let mut start_txs = Vec::with_capacity(TRACKS);
        let mut final_rxs = Vec::with_capacity(TRACKS);

        for _ in 0..TRACKS {
            let (start_tx, mut chain_rx) = sync::mpsc::channel(10);

            for _ in 0..CHAIN {
                let (next_tx, next_rx) = sync::mpsc::channel(10);

                // Forward all the messages
                rt.spawn(async move {
                    while let Some(v) = chain_rx.recv().await {
                        next_tx.send(v).await.unwrap();
                    }
                });

                chain_rx = next_rx;
            }

            // This final task cycles if needed
            let (final_tx, final_rx) = sync::mpsc::channel(10);
            let cycle_tx = start_tx.clone();
            let mut rem = CYCLES;

            rt.spawn(async move {
                for _ in 0..CYCLES {
                    let msg = chain_rx.recv().await.unwrap();

                    rem -= 1;

                    if rem == 0 {
                        final_tx.send(msg).await.unwrap();
                    } else {
                        cycle_tx.send(msg).await.unwrap();
                    }
                }
            });

            start_txs.push(start_tx);
            final_rxs.push(final_rx);
        }

        {
            rt.block_on(async move {
                for start_tx in start_txs {
                    start_tx.send("ping").await.unwrap();
                }

                for mut final_rx in final_rxs {
                    final_rx.recv().await.unwrap();
                }
            });
        }
    }
}

#[test]
fn lifo_slot_budget() {
    async fn my_fn() {
        spawn_another();
    }

    fn spawn_another() {
        nio::spawn(my_fn());
    }

    let rt = runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .unwrap();

    let (send, recv) = oneshot::channel();

    rt.spawn(async move {
        nio::spawn(my_fn());
        let _ = send.send(());
    });

    let _ = rt.block_on(recv);
}

#[test]
fn spawn_shutdown() {
    let rt = rt();
    let (tx, rx) = mpsc::channel();

    rt.block_on(async {
        nio::spawn(client_server(tx.clone()));
    });

    // Use spawner
    rt.spawn(client_server(tx));

    assert_ok!(rx.recv());
    assert_ok!(rx.recv());

    drop(rt);
    assert_err!(rx.try_recv());
}

async fn client_server(tx: mpsc::Sender<()>) {
    let server = assert_ok!(TcpListener::bind("127.0.0.1:0").await);

    // Get the assigned address
    let addr = assert_ok!(server.local_addr());

    // Spawn the server
    nio::spawn(async move {
        // Accept a socket
        let (mut socket, _) = server.accept().await.unwrap();

        // Write some data
        socket.write_all(b"hello").await.unwrap();
    });

    let mut client = TcpStream::connect(&addr).await.unwrap();

    let mut buf = vec![];
    client.read_to_end(&mut buf).await.unwrap();

    assert_eq!(buf, b"hello");
    tx.send(()).unwrap();
}

#[test]
fn multi_threadpool() {
    use sync::oneshot;

    let rt1 = rt();
    let rt2 = rt();

    let (tx, rx) = oneshot::channel();
    let (done_tx, done_rx) = mpsc::channel();

    rt2.spawn(async move {
        rx.await.unwrap();
        done_tx.send(()).unwrap();
    });

    rt1.spawn(async move {
        tx.send(()).unwrap();
    });

    done_rx.recv().unwrap();
}

#[test]
fn yield_after_block_in_place() {
    let rt = nio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .build()
        .unwrap();

    rt.block_on(async {
        nio::spawn(async move {
            // Block in place then enter a new runtime
            nio::task::spawn_blocking(|| {
                let rt = nio::runtime::Builder::new_multi_thread().build().unwrap();

                rt.block_on(async {});
            });

            // Yield, then complete
            nio::task::yield_now().await;
        })
        .await
        .unwrap()
    });
}

// Testing this does not panic
#[test]
fn max_blocking_threads() {
    let _rt = nio::runtime::Builder::new_multi_thread()
        .max_blocking_threads(1)
        .build()
        .unwrap();
}

#[test]
#[should_panic]
fn max_blocking_threads_set_to_zero() {
    let _rt = nio::runtime::Builder::new_multi_thread()
        .max_blocking_threads(0)
        .build()
        .unwrap();
}

#[nio::test(worker_threads = 2)]
async fn hang_on_shutdown() {
    let (sync_tx, sync_rx) = std::sync::mpsc::channel::<()>();
    nio::spawn(async move {
        nio::task::spawn_blocking(move || {
            sync_rx.recv().ok();
        });
    });

    nio::spawn(async {
        nio::time::sleep(std::time::Duration::from_secs(2)).await;
        drop(sync_tx);
    });
    nio::time::sleep(std::time::Duration::from_secs(1)).await;
}

/// Demonstrates nio-rs/nio#3869
#[test]
fn wake_during_shutdown() {
    struct Shared {
        waker: Option<Waker>,
    }

    struct MyFuture {
        shared: Arc<Mutex<Shared>>,
        put_waker: bool,
    }

    impl MyFuture {
        fn new() -> (Self, Self) {
            let shared = Arc::new(Mutex::new(Shared { waker: None }));
            let f1 = MyFuture {
                shared: shared.clone(),
                put_waker: true,
            };
            let f2 = MyFuture {
                shared,
                put_waker: false,
            };
            (f1, f2)
        }
    }

    impl Future for MyFuture {
        type Output = ();

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            let me = Pin::into_inner(self);
            let mut lock = me.shared.lock().unwrap();
            if me.put_waker {
                lock.waker = Some(cx.waker().clone());
            }
            Poll::Pending
        }
    }

    impl Drop for MyFuture {
        fn drop(&mut self) {
            let mut lock = self.shared.lock().unwrap();
            if !self.put_waker {
                lock.waker.take().unwrap().wake();
            }
            drop(lock);
        }
    }

    let rt = nio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    let (f1, f2) = MyFuture::new();

    rt.spawn(f1);
    rt.spawn(f2);

    rt.block_on(async { nio::time::sleep(Duration::from_millis(20)).await });
}

// Testing the tuning logic is tricky as it is inherently timing based, and more
// of a heuristic than an exact behavior. This test checks that the interval
// changes over time based on load factors. There are no assertions, completion
// is sufficient. If there is a regression, this test will hang. In theory, we
// could add limits, but that would be likely to fail on CI.
#[test]
fn test_tuning() {
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;

    let rt = runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .build()
        .unwrap();

    fn iter(flag: Arc<AtomicBool>, counter: Arc<AtomicUsize>, stall: bool) {
        if flag.load(Relaxed) {
            if stall {
                std::thread::sleep(Duration::from_micros(5));
            }

            counter.fetch_add(1, Relaxed);
            nio::spawn(async move { iter(flag, counter, stall) });
        }
    }

    let flag = Arc::new(AtomicBool::new(true));
    let counter = Arc::new(AtomicUsize::new(61));
    let interval = Arc::new(AtomicUsize::new(61));

    {
        let flag = flag.clone();
        let counter = counter.clone();
        rt.spawn(async move { iter(flag, counter, true) });
    }

    // Now, hammer the injection queue until the interval drops.
    let mut n = 0;
    loop {
        let curr = interval.load(Relaxed);

        if curr <= 8 {
            n += 1;
        } else {
            n = 0;
        }

        // Make sure we get a few good rounds. Jitter in the tuning could result
        // in one "good" value without being representative of reaching a good
        // state.
        if n == 3 {
            break;
        }

        if Arc::strong_count(&interval) < 5_000 {
            let counter = counter.clone();
            let interval = interval.clone();

            rt.spawn(async move {
                let prev = counter.swap(0, Relaxed);
                interval.store(prev, Relaxed);
            });

            std::thread::yield_now();
        }
    }

    flag.store(false, Relaxed);

    let w = Arc::downgrade(&interval);
    drop(interval);

    while w.strong_count() > 0 {
        std::thread::sleep(Duration::from_micros(500));
    }

    // Now, run it again with a faster task
    let flag = Arc::new(AtomicBool::new(true));
    // Set it high, we know it shouldn't ever really be this high
    let counter = Arc::new(AtomicUsize::new(10_000));
    let interval = Arc::new(AtomicUsize::new(10_000));

    {
        let flag = flag.clone();
        let counter = counter.clone();
        rt.spawn(async move { iter(flag, counter, false) });
    }

    // Now, hammer the injection queue until the interval reaches the expected range.
    let mut n = 0;
    loop {
        let curr = interval.load(Relaxed);

        if curr <= 1_000 && curr > 32 {
            n += 1;
        } else {
            n = 0;
        }

        if n == 3 {
            break;
        }

        if Arc::strong_count(&interval) <= 5_000 {
            let counter = counter.clone();
            let interval = interval.clone();

            rt.spawn(async move {
                let prev = counter.swap(0, Relaxed);
                interval.store(prev, Relaxed);
            });
        }

        std::thread::yield_now();
    }

    flag.store(false, Relaxed);
}

fn rt() -> runtime::Runtime {
    runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
}
