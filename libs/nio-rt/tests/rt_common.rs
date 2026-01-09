#![allow(clippy::needless_range_loop)]
#![warn(rust_2018_idioms)]

use nio_future::yield_now;
use nio_rt::{Runtime, RuntimeBuilder, sleep, spawn, spawn_local};
use std::thread;
use std::time::Duration;

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
    const ITER: usize = 200;

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
    const ITER: usize = 500;

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

// #[test]
// fn complete_block_on_under_load() {
//     for rt in CORES.map(rt) {
//         rt.block_on(|| async {
//             let (tx, rx) = oneshot::channel();

//             // Spin hard
//             spawn(async {
//                 loop {
//                     yield_now().await;
//                 }
//             });

//             spawn_local(async {
//                 sleep(Duration::from_millis(100)).await;
//                 assert_ok!(tx.send(()));
//             });

//             assert_ok!(rx.await);
//         });
//     }
// }

// #[test]
// fn test_name() {
//    for rt in CORES.map(rt) {
//         // ...
//    }
// }
