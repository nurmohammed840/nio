// #![allow(warnings)]
use criterion::measurement::WallTime;
use criterion::{criterion_group, criterion_main, BenchmarkGroup, Criterion};
use futures::{join, try_join};

use import::channel::*;
use import::rt::*;

mod import {
    pub mod channel;
    pub mod rt;
    // pub mod rt {
    //     pub mod tokio;
    //     pub use tokio::*;
    // }
}

criterion_main!(contention, uncontented);
criterion_group!(contention, bench_contention);
criterion_group!(uncontented, bench_uncontented);

fn single_rt() -> Runtime {
    Runtime::new(1)
}

fn multi_rt() -> Runtime {
    Runtime::new(6)
}

fn bench_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("contention");
    contended_concurrent_multi(&mut group);
    contended_concurrent_single(&mut group);
    group.finish();
}

fn bench_uncontented(c: &mut Criterion) {
    let mut group = c.benchmark_group("uncontented");
    uncontended(&mut group);
    uncontended_concurrent_multi(&mut group);
    uncontended_concurrent_single(&mut group);
    group.finish();
}

fn uncontended(g: &mut BenchmarkGroup<WallTime>) {
    let rt = multi_rt();

    static S: Semaphore = Semaphore::const_new(10);
    g.bench_function("multi", |b| {
        b.iter(|| {
            rt.block_on(async move {
                for _ in 0..6 {
                    let permit = S.acquire().await;
                    drop(permit);
                }
            })
        })
    });
}

fn uncontended_concurrent_multi(g: &mut BenchmarkGroup<WallTime>) {
    let rt = multi_rt();

    static S: Semaphore = Semaphore::const_new(10);
    g.bench_function("concurrent_multi", |b| {
        b.iter(|| {
            rt.block_on(async move {
                let j = try_join! {
                    spawn(task(&S)),
                    spawn(task(&S)),
                    spawn(task(&S)),
                    spawn(task(&S)),
                    spawn(task(&S)),
                    spawn(task(&S))
                };
                j.unwrap();
            })
        })
    });
}

fn uncontended_concurrent_single(g: &mut BenchmarkGroup<WallTime>) {
    let rt = single_rt();

    static S: Semaphore = Semaphore::const_new(10);
    g.bench_function("concurrent_single", |b| {
        b.iter(|| {
            rt.block_on(async move {
                join! {
                    task(&S),
                    task(&S),
                    task(&S),
                    task(&S),
                    task(&S),
                    task(&S)
                };
            })
        })
    });
}

fn contended_concurrent_multi(g: &mut BenchmarkGroup<WallTime>) {
    let rt = multi_rt();

    static S: Semaphore = Semaphore::const_new(5);
    g.bench_function("concurrent_multi", |b| {
        b.iter(|| {
            rt.block_on(async move {
                let j = try_join! {
                    spawn(task(&S)),
                    spawn(task(&S)),
                    spawn(task(&S)),
                    spawn(task(&S)),
                    spawn(task(&S)),
                    spawn(task(&S))
                };
                j.unwrap();
            })
        })
    });
}

fn contended_concurrent_single(g: &mut BenchmarkGroup<WallTime>) {
    let rt = single_rt();

    static S: Semaphore = Semaphore::const_new(5);
    g.bench_function("concurrent_single", |b| {
        b.iter(|| {
            rt.block_on(async move {
                join! {
                    task(&S),
                    task(&S),
                    task(&S),
                    task(&S),
                    task(&S),
                    task(&S)
                };
            })
        })
    });
}

async fn task(s: &Semaphore) {
    let permit = s.acquire().await;
    drop(permit);
}
