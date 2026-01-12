use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::{Duration, Instant};

use import::rt::*;

mod import {
    pub mod rt;

    // pub mod rt {
    //     pub mod tokio;
    //     pub use tokio::*;
    // }
}

// a very quick async task, but might timeout
async fn quick_job() -> usize {
    1
}

fn single_thread_scheduler_timeout(c: &mut Criterion) {
    do_timeout_test(c, 1, "single_thread_timeout");
}

fn multi_thread_scheduler_timeout(c: &mut Criterion) {
    do_timeout_test(c, 8, "multi_thread_timeout-8");
}

fn do_timeout_test(c: &mut Criterion, workers: usize, name: &str) {
    let runtime = Runtime::new(workers);
    c.bench_function(name, |b| {
        b.iter_custom(|iters| {
            let start = Instant::now();
            runtime.block_on(async move {
                black_box(spawn_timeout_job(iters as usize, workers)).await;
            });
            start.elapsed()
        })
    });
}

async fn spawn_timeout_job(iters: usize, procs: usize) {
    let mut handles = Vec::with_capacity(procs);
    for _ in 0..procs {
        handles.push(spawn_pinned(|| async move {
            for _ in 0..iters / procs {
                let h = timeout(Duration::from_secs(1), quick_job());
                assert_eq!(black_box(h.await.unwrap()), 1);
            }
        }));
    }
    for handle in handles {
        handle.await.unwrap();
    }
}

fn single_thread_scheduler_sleep(c: &mut Criterion) {
    do_sleep_test(c, 1, "single_thread_sleep");
}

fn multi_thread_scheduler_sleep(c: &mut Criterion) {
    do_sleep_test(c, 8, "multi_thread_sleep-8");
}

fn do_sleep_test(c: &mut Criterion, workers: usize, name: &str) {
    let runtime = Runtime::new(workers);

    c.bench_function(name, |b| {
        b.iter_custom(|iters| {
            let start = Instant::now();
            runtime.block_on(async move {
                black_box(spawn_sleep_job(iters as usize, workers)).await;
            });
            start.elapsed()
        })
    });
}

async fn spawn_sleep_job(iters: usize, procs: usize) {
    let mut handles = Vec::with_capacity(procs);
    for _ in 0..procs {
        handles.push(spawn(async move {
            for _ in 0..iters / procs {
                let _h = black_box(sleep(Duration::from_secs(1)));
            }
        }));
    }
    for handle in handles {
        handle.await.unwrap();
    }
}

criterion_group!(
    timeout_benchmark,
    single_thread_scheduler_timeout,
    multi_thread_scheduler_timeout,
    single_thread_scheduler_sleep,
    multi_thread_scheduler_sleep
);

criterion_main!(timeout_benchmark);
