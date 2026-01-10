use nio_rt::{Runtime, RuntimeBuilder, spawn};

use std::{
    future::{Future, poll_fn},
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering::Relaxed},
        mpsc,
    },
    task::{Context, Poll, Waker},
};

mod support {
    pub mod futures;
}
use support::futures;
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
    const NUM: usize = 1_000;

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
