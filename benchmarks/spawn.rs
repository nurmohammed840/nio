use criterion::{black_box, criterion_group, criterion_main, Criterion};

use import::rt::{self, *};
use import::task::yield_now;

mod import {
    pub mod task;

    pub mod rt;
    // pub mod rt {
    //     pub mod tokio;
    //     pub use tokio::*;
    // }
}

async fn work() -> usize {
    let val = 1 + 1;
    yield_now().await;
    black_box(val)
}

fn basic_scheduler_spawn(c: &mut Criterion) {
    let runtime = Runtime::new(1);

    c.bench_function("basic_scheduler_spawn", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let h = rt::spawn(work());
                assert_eq!(h.await.unwrap(), 2);
            });
        })
    });
}

fn basic_scheduler_spawn10(c: &mut Criterion) {
    let runtime = Runtime::new(1);

    c.bench_function("basic_scheduler_spawn10", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let mut handles = Vec::with_capacity(10);
                for _ in 0..10 {
                    handles.push(rt::spawn(work()));
                }
                for handle in handles {
                    assert_eq!(handle.await.unwrap(), 2);
                }
            });
        })
    });
}

fn threaded_scheduler_spawn(c: &mut Criterion) {
    let runtime = Runtime::new(2);

    c.bench_function("threaded_scheduler_spawn", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let h = rt::spawn(work());
                assert_eq!(h.await.unwrap(), 2);
            });
        })
    });
}

fn threaded_scheduler_spawn10(c: &mut Criterion) {
    let runtime = Runtime::new(2);

    c.bench_function("threaded_scheduler_spawn10", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let mut handles = Vec::with_capacity(10);
                for _ in 0..10 {
                    handles.push(rt::spawn(work()));
                }
                for handle in handles {
                    assert_eq!(handle.await.unwrap(), 2);
                }
            });
        })
    });
}

criterion_group!(
    spawn,
    basic_scheduler_spawn,
    basic_scheduler_spawn10,
    threaded_scheduler_spawn,
    threaded_scheduler_spawn10,
);

criterion_main!(spawn);
