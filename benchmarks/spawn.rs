use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

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

const COUNT: [usize; 4] = [10, 100, 1000, 10_000];

fn basic_scheduler_spawn_many(c: &mut Criterion) {
    let runtime = Runtime::new(1);
    for count in COUNT {
        c.bench_function(&format!("basic_scheduler_spawn_{count}"), |b| {
            b.iter(|| {
                runtime.block_on(async move {
                    let mut handles = Vec::with_capacity(count);
                    for _ in 0..count {
                        handles.push(rt::spawn(work()));
                    }
                    for handle in handles {
                        assert_eq!(handle.await.unwrap(), 2);
                    }
                });
            })
        });
    }
}

fn threaded_scheduler_spawn_many(c: &mut Criterion) {
    let runtime = Runtime::new(2);
    for count in COUNT {
        c.bench_function(&format!("threaded_scheduler_spawn_{count}"), |b| {
            b.iter(|| {
                runtime.block_on(async move {
                    let mut handles = Vec::with_capacity(count);
                    for _ in 0..count {
                        handles.push(rt::spawn(work()));
                    }
                    for handle in handles {
                        assert_eq!(handle.await.unwrap(), 2);
                    }
                });
            })
        });
    }
}

criterion_group!(
    spawn,
    basic_scheduler_spawn_many,
    threaded_scheduler_spawn_many,
);

criterion_main!(spawn);
