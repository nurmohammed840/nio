use nio_rt::{Runtime, RuntimeBuilder, spawn};

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering::Relaxed},
    mpsc,
};

mod support {
    pub mod futures;
}
// use support::futures;
use support::futures::sync::oneshot;
use support::futures::test::assert_ok;

// fn rt(core: u8) -> Runtime {
//     RuntimeBuilder::new().worker_threads(core).rt().unwrap()
// }

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

// #[test]
// fn many_multishot_futures() {
//     const CHAIN: usize = 200;
//     const CYCLES: usize = 5;
//     const TRACKS: usize = 50;

//     let rt = multi();
//     let ctx = rt.context();
//     let mut start_txs = Vec::with_capacity(TRACKS);
//     let mut final_rxs = Vec::with_capacity(TRACKS);

//     for _ in 0..TRACKS {
//         let (start_tx, mut chain_rx) = futures::sync::mpsc::channel(10);

//         for _ in 0..CHAIN {
//             let (next_tx, next_rx) = futures::sync::mpsc::channel(10);

//             // Forward all the messages
//             ctx.spawn(async move {
//                 while let Some(v) = chain_rx.recv().await {
//                     next_tx.send(v).await.unwrap();
//                 }
//             });

//             chain_rx = next_rx;
//         }

//         // This final task cycles if needed
//         let (final_tx, final_rx) = futures::sync::mpsc::channel(10);
//         let cycle_tx = start_tx.clone();
//         let mut rem = CYCLES;

//         ctx.spawn(async move {
//             for _ in 0..CYCLES {
//                 let msg = chain_rx.recv().await.unwrap();

//                 rem -= 1;

//                 if rem == 0 {
//                     final_tx.send(msg).await.unwrap();
//                 } else {
//                     cycle_tx.send(msg).await.unwrap();
//                 }
//             }
//         });

//         start_txs.push(start_tx);
//         final_rxs.push(final_rx);
//     }

//     rt.block_on(|| async move {
//         for start_tx in start_txs {
//             start_tx.send("ping").await.unwrap();
//         }

//         for mut final_rx in final_rxs {
//             final_rx.recv().await.unwrap();
//         }
//     });
// }
