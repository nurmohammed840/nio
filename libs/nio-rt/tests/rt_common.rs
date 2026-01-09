#![allow(clippy::needless_range_loop)]
#![warn(rust_2018_idioms)]

use nio_future::{block_on, yield_now};
use nio_rt::net::{TcpListener, TcpStream};
use nio_rt::{
    Runtime, RuntimeBuilder, Sleep, sleep, spawn, spawn_blocking, spawn_local, spawn_pinned_at,
};
use std::time::Duration;
use std::{thread, time::Instant};

use tokio::sync::{mpsc, oneshot};
use tokio_test::assert_ok;

fn rt(core: u8) -> Runtime {
    RuntimeBuilder::new().worker_threads(core).rt().unwrap()
}

const CORES: [u8; 3] = [1, 2, 4];

#[test]
fn block_on_sync() {
    let mut win = String::new();

    let win = rt(1).block_on(|| async move {
        win += "Done";
        win
    });

    assert_eq!(win, "Done");
}

#[test]
fn block_on_async() {
    let out = rt(1).block_on(|| async {
        let (tx, rx) = oneshot::channel();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            tx.send("ZOMG").unwrap();
        });
        assert_ok!(rx.await)
    });
    assert_eq!(out, "ZOMG");
}

#[test]
fn spawn_one_bg() {
    for rt in CORES.map(rt) {
        let out = rt.block_on(|| async {
            let (tx, rx) = oneshot::channel();
            spawn(async move {
                tx.send("ZOMG").unwrap();
            });
            assert_ok!(rx.await)
        });
        assert_eq!(out, "ZOMG");
    }
}

#[test]
fn spawn_one_join() {
    for rt in CORES.map(rt) {
        let out = rt.block_on(|| async {
            let (tx, rx) = oneshot::channel();

            let handle = spawn(async move {
                tx.send("ZOMG").unwrap();
                "DONE"
            });

            let msg = assert_ok!(rx.await);

            let out = assert_ok!(handle.await);
            assert_eq!(out, "DONE");

            msg
        });
        assert_eq!(out, "ZOMG");
    }
}

#[test]
fn spawn_two() {
    for rt in CORES.map(rt) {
        let out = rt.block_on(|| async {
            let (tx1, rx1) = oneshot::channel();
            let (tx2, rx2) = oneshot::channel();

            spawn(async move {
                assert_ok!(tx1.send("ZOMG"));
            });

            spawn(async move {
                let msg = assert_ok!(rx1.await);
                assert_ok!(tx2.send(msg));
            });

            assert_ok!(rx2.await)
        });
        assert_eq!(out, "ZOMG");
    }
}

#[test]
fn spawn_many_from_block_on() {
    const ITER: usize = if cfg!(miri) { 20 } else { 500 };

    for rt in CORES.map(rt) {
        let out = rt.block_on(|| async {
            let (done_tx, mut done_rx) = mpsc::unbounded_channel();

            let mut txs = (0..ITER)
                .map(|i| {
                    let (tx, rx) = oneshot::channel();
                    let done_tx = done_tx.clone();

                    spawn(async move {
                        let msg = assert_ok!(rx.await);
                        assert_eq!(i, msg);
                        assert_ok!(done_tx.send(msg));
                    });

                    tx
                })
                .collect::<Vec<_>>();

            drop(done_tx);

            thread::spawn(move || {
                for (i, tx) in txs.drain(..).enumerate() {
                    assert_ok!(tx.send(i));
                }
            });

            let mut out = vec![];
            while let Some(i) = done_rx.recv().await {
                out.push(i);
            }

            out.sort_unstable();
            out
        });

        assert_eq!(ITER, out.len());

        for i in 0..ITER {
            assert_eq!(i, out[i]);
        }
    }
}

#[test]
fn spawn_many_from_task() {
    const ITER: usize = if cfg!(miri) { 20 } else { 500 };

    for rt in CORES.map(rt) {
        let out = rt.block_on(|| async {
            spawn(async move {
                let (done_tx, mut done_rx) = mpsc::unbounded_channel();

                let mut txs = (0..ITER)
                    .map(|i| {
                        let (tx, rx) = oneshot::channel();
                        let done_tx = done_tx.clone();

                        spawn(async move {
                            let msg = assert_ok!(rx.await);
                            assert_eq!(i, msg);
                            assert_ok!(done_tx.send(msg));
                        });

                        tx
                    })
                    .collect::<Vec<_>>();

                drop(done_tx);

                thread::spawn(move || {
                    for (i, tx) in txs.drain(..).enumerate() {
                        assert_ok!(tx.send(i));
                    }
                });

                let mut out = vec![];
                while let Some(i) = done_rx.recv().await {
                    out.push(i);
                }

                out.sort_unstable();
                out
            })
            .await
            .unwrap()
        });

        assert_eq!(ITER, out.len());

        for i in 0..ITER {
            assert_eq!(i, out[i]);
        }
    }
}

#[test]
fn spawn_one_from_block_on_called_on_handle() {
    for rt in CORES.map(rt) {
        let (tx, rx) = oneshot::channel();

        let handle = rt.block_on(|| async {
            spawn(async move {
                tx.send("ZOMG").unwrap();
                "DONE"
            })
        });

        let out = rt.block_on(|| async {
            let msg = assert_ok!(rx.await);

            let out = assert_ok!(handle.await);
            assert_eq!(out, "DONE");

            msg
        });

        assert_eq!(out, "ZOMG");
    }
}

#[test]
fn spawn_await_chain() {
    for rt in CORES.map(rt) {
        let out = rt.block_on(|| async {
            assert_ok!(spawn(async { assert_ok!(spawn(async { "hello" }).await) }).await)
        });
        assert_eq!(out, "hello");
    }
}

#[test]
fn nested_rt() {
    let out = rt(1).block_on(|| async move { rt(1).block_on(|| async { "hello" }) });
    assert_eq!(out, "hello");
}

#[test]
fn create_rt_in_block_on() {
    let rt1 = rt(1);
    let rt2 = rt1.block_on(|| async { rt(1) });
    let out = rt2.block_on(|| async { "ZOMG" });

    assert_eq!(out, "ZOMG");
}

#[test]
fn complete_task_under_load() {
    for rt in CORES.map(rt) {
        rt.block_on(|| async {
            let (tx1, rx1) = oneshot::channel();
            let (tx2, rx2) = oneshot::channel();

            // Spin hard
            spawn(async {
                loop {
                    yield_now().await;
                }
            });

            thread::spawn(move || {
                thread::sleep(Duration::from_millis(50));
                assert_ok!(tx1.send(()));
            });

            spawn(async move {
                assert_ok!(rx1.await);
                assert_ok!(tx2.send(()));
            });

            assert_ok!(rx2.await);
        });
    }
}

#[test]
fn spawn_from_other_thread_idle() {
    for rt in CORES.map(rt) {
        let (tx, rx) = oneshot::channel();
        let handle = rt.context();

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));

            handle.spawn(async move {
                assert_ok!(tx.send(()));
            });
        });

        rt.block_on(|| async move {
            assert_ok!(rx.await);
        });
    }
}

#[test]
fn spawn_from_other_thread_under_load() {
    for rt in CORES.map(rt) {
        let handle = rt.context();

        let (tx, rx) = oneshot::channel();

        thread::spawn(move || {
            handle.spawn(async move {
                assert_ok!(tx.send(()));
            });
        });

        rt.block_on(|| async move {
            // Spin hard
            spawn(async {
                loop {
                    yield_now().await;
                }
            });

            assert_ok!(rx.await);
        });
    }
}

#[test]
fn sleep_at_root() {
    for rt in CORES.map(rt) {
        let now = Instant::now();
        let dur = Duration::from_millis(50);

        rt.block_on(|| async move {
            Sleep::at(now + dur).await;
        });

        assert!(now.elapsed() >= dur);
    }
}

#[test]
fn sleep_in_spawn() {
    for rt in CORES.map(rt) {
        let now = Instant::now();
        let dur = Duration::from_millis(50);

        rt.block_on(|| async move {
            let (tx, rx) = oneshot::channel();

            spawn_local(async move {
                Sleep::at(now + dur).await;
                assert_ok!(tx.send(()));
            });

            assert_ok!(rx.await);
        });

        assert!(now.elapsed() >= dur);
    }
}

#[test]
#[cfg(not(miri))]
fn block_on_socket() {
    for rt in CORES.map(rt) {
        rt.block_on(|| async {
            let (tx, rx) = oneshot::channel();

            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();

            spawn_local(async move {
                let _ = listener.accept().await;
                tx.send(()).unwrap();
            });

            TcpStream::connect(&addr).await.unwrap();
            rx.await.unwrap();
        });
    }
}

#[test]
fn spawn_from_blocking() {
    for rt in CORES.map(rt) {
        let ctx = rt.context();
        rt.block_on(|| async move {
            let inner =
                assert_ok!(spawn_blocking(move || { ctx.spawn(async move { "hello" }) }).await);

            assert_ok!(inner.await)
        });
    }
}

#[test]
fn spawn_blocking_from_blocking() {
    for rt in CORES.map(rt) {
        let ctx = rt.context();
        let out = rt.block_on(|| async move {
            let inner =
                assert_ok!(spawn_blocking(move || { ctx.spawn_blocking(|| "hello") }).await);

            assert_ok!(inner.await)
        });
        assert_eq!(out, "hello");
    }
}

#[test]
fn always_active_parker() {
    for rt in CORES.map(rt) {
        let ctx = rt.context();

        let (tx1, rx1) = oneshot::channel();
        let (tx2, rx2) = oneshot::channel();

        let jh1 = thread::spawn(move || {
            rt.block_on(|| async {
                rx2.await.unwrap();
                sleep(Duration::from_millis(5)).await;
                tx1.send(()).unwrap();
            });
        });

        let jh2 = thread::spawn(move || {
            block_on(ctx.spawn_pinned(|| async {
                tx2.send(()).unwrap();
                sleep(Duration::from_millis(5)).await;
                rx1.await.unwrap();
                sleep(Duration::from_millis(5)).await;
            }))
            .unwrap();
        });

        jh1.join().unwrap();
        jh2.join().unwrap();
    }
}

#[test]
#[cfg(not(miri))]
fn io_driver_called_when_under_load() {
    use futures_lite::{AsyncReadExt, AsyncWriteExt};

    for rt in CORES.map(rt) {
        let ctx = rt.context();
        // Create a lot of constant load. The scheduler will always be busy.
        for _ in 0..100 {
            ctx.spawn(async {
                loop {
                    yield_now().await;
                }
            });
        }

        rt.block_on(|| async {
            let listener = assert_ok!(TcpListener::bind("127.0.0.1:0").await);
            let addr = assert_ok!(listener.local_addr());

            let srv = spawn_local(async move {
                let conn = assert_ok!(listener.accept().await);
                let mut stream = assert_ok!(conn.connect().await);
                assert_ok!(stream.write_all(b"hello world").await);
            });

            let cli = spawn_local(async move {
                let mut stream = assert_ok!(TcpStream::connect(addr).await);
                let mut dst = vec![0; 11];

                assert_ok!(stream.read_exact(&mut dst).await);
                assert_eq!(dst, b"hello world");
            });

            assert_ok!(srv.await);
            assert_ok!(cli.await);
        });
    }
}


// #[test]
// fn test_name() {
//     for rt in CORES.map(rt) {
//         rt.block_on(|| async {
//             // ...
//         });
//     }
// }
