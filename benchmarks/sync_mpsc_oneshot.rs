use criterion::{criterion_group, criterion_main, Criterion};

use futures::{
    channel::{mpsc, oneshot},
    SinkExt, StreamExt,
};

use import::rt::*;
mod import {
    pub mod rt;
    // pub mod rt {
    //     pub mod tokio;
    //     pub use tokio::*;
    // }
}

criterion_main!(sync_mpsc_oneshot_group);
criterion_group!(
    sync_mpsc_oneshot_group,
    request_reply_current_thread,
    request_reply_multi_threaded,
);

fn request_reply_current_thread(c: &mut Criterion) {
    request_reply(c, Runtime::new(1));
}

fn request_reply_multi_threaded(c: &mut Criterion) {
    request_reply(c, Runtime::new(2));
}

fn request_reply(b: &mut Criterion, rt: Runtime) {
    let tx = rt.block_on(async move {
        let (tx, mut rx) = mpsc::channel::<oneshot::Sender<()>>(10);
        spawn(async move {
            while let Some(reply) = rx.next().await {
                reply.send(()).unwrap();
            }
        });
        tx
    });

    b.bench_function("request_reply", |b| {
        b.iter(|| {
            let mut task_tx = tx.clone();
            rt.block_on(async move {
                for _ in 0..1_000 {
                    let (o_tx, o_rx) = oneshot::channel();
                    task_tx.send(o_tx).await.unwrap();
                    let _ = o_rx.await;
                }
            })
        })
    });
}
