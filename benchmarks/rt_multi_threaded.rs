use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::sync::mpsc;
// use std::time::{Duration, Instant};

use import::rt::*;
mod import {
    pub mod rt;
    // pub mod rt {
    //     pub mod tokio;
    //     pub use tokio::*;
    // }
}

const NUM_WORKERS: usize = 4;
const NUM_SPAWN: usize = 10_000;
// const STALL_DUR: Duration = Duration::from_micros(10);

fn rt_multi_spawn_many_local(c: &mut Criterion) {
    let rt = Runtime::new(NUM_WORKERS);
    let ctx = rt.handle();

    let m = rt.0.context().metrics();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(3));
        let count: Vec<_> = m.task_counts().collect();
        println!("count: {:#?}", count);
    });

    let (tx, rx) = mpsc::sync_channel(1000);
    static REM: AtomicUsize = AtomicUsize::new(0);

    c.bench_function("spawn_many_local", |b| {
        b.iter(|| {
            REM.store(NUM_SPAWN, Relaxed);

            for _ in 0..NUM_SPAWN {
                let tx = tx.clone();
                ctx.spawn(async move {
                    if 1 == REM.fetch_sub(1, Relaxed) {
                        tx.send(()).unwrap();
                    }
                });
            }
            rx.recv().unwrap();
        })
    });
}

criterion_group!(
    rt_multi_scheduler,
    rt_multi_spawn_many_local,
    // rt_multi_spawn_many_remote_idle,
    // rt_multi_spawn_many_remote_busy1,
    // rt_multi_spawn_many_remote_busy2,
    // rt_multi_ping_pong,
    // rt_multi_yield_many,
    // rt_multi_chained_spawn,
);

criterion_main!(rt_multi_scheduler);
