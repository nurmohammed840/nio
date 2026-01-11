use nio_rt::{Runtime, RuntimeBuilder, spawn};

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering::Relaxed},
    mpsc,
};

mod support {
    pub mod futures;
}
use support::futures;
use support::futures::sync::oneshot;
use support::futures::test::assert_ok;

fn rt(core: u8) -> Runtime {
    RuntimeBuilder::new().worker_threads(core).rt().unwrap()
}

fn multi() -> Runtime {
    RuntimeBuilder::new().rt().unwrap()
}

#[test]
fn many_oneshot_futures() {
    // used for notifying the main thread
    const NUM: usize = if cfg!(miri) { 20 } else { 1_000 };

    for _ in 0..5 {
        let (tx, rx) = mpsc::channel();

        let rt = multi().context();
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
    }
}

#[test]
fn spawn_two() {
    let rt = multi();

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

#[test]
fn many_multishot_futures() {
    const CHAIN: usize = if cfg!(miri) { 20 } else { 200 };
    const CYCLES: usize = if cfg!(miri) { 3 } else { 5 };
    const TRACKS: usize = if cfg!(miri) { 2 } else { 50 };

    let rt = multi();
    let mut start_txs = Vec::with_capacity(TRACKS);
    let mut final_rxs = Vec::with_capacity(TRACKS);

    for _ in 0..TRACKS {
        let (start_tx, mut chain_rx) = futures::sync::mpsc::channel(10);

        for _ in 0..CHAIN {
            let (next_tx, next_rx) = futures::sync::mpsc::channel(10);

            // Forward all the messages
            rt.spawn(async move {
                while let Some(v) = chain_rx.recv().await {
                    next_tx.send(v).await.unwrap();
                }
            });

            chain_rx = next_rx;
        }

        // This final task cycles if needed
        let (final_tx, final_rx) = futures::sync::mpsc::channel(10);
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

    rt.block_on(|| async move {
        for start_tx in start_txs {
            start_tx.send("ping").await.unwrap();
        }

        for mut final_rx in final_rxs {
            final_rx.recv().await.unwrap();
        }
    });
}

#[test]
fn lifo_slot_budget() {
    async fn my_fn() {
        spawn_another();
    }

    fn spawn_another() {
        spawn(my_fn());
    }

    let rt = multi();

    let (send, recv) = oneshot::channel();

    rt.spawn(async move {
        spawn(my_fn());
        let _ = send.send(());
    });

    let _ = rt.block_on(|| recv);
}

#[test]
fn blocking() {
    // used for notifying the main thread
    const NUM: usize = if cfg!(miri) { 50 } else { 1_000 };

    let (tx, rx) = mpsc::channel();

    let rt = multi();
    let cnt = Arc::new(AtomicUsize::new(0));

    // there are four workers in the pool
    // so, if we run 4 blocking tasks, we know that handoff must have happened
    let block = Arc::new(std::sync::Barrier::new(5));
    for _ in 0..4 {
        let block = block.clone();

        let ctx = rt.context();
        rt.spawn(async move {
            ctx.spawn_blocking(move || {
                block.wait();
                block.wait();
            })
        });
    }
    block.wait();

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
    block.wait();
}

#[test]
fn multi_threadpool() {
    let rt1 = multi();
    let rt2 = multi();

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

