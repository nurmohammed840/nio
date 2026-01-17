use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion,
};
use futures::{channel::mpsc, SinkExt, StreamExt};

use import::rt::*;
mod import {
    pub mod rt;
    // pub mod rt {
    //     pub mod tokio;
    //     pub use tokio::*;
    // }
}

const NUM_WORKERS: usize = 6;

fn rt() -> Runtime {
    Runtime::new(NUM_WORKERS)
}

criterion_main!(contention, uncontented);
criterion_group!(contention, bench_contention);
criterion_group!(uncontented, bench_uncontented);

fn bench_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("contention");
    contention_unbounded(&mut group);
    group.finish();
}

fn bench_uncontented(c: &mut Criterion) {
    let mut group = c.benchmark_group("uncontented");
    uncontented_unbounded(&mut group);
    group.finish();
}

fn uncontented_unbounded(g: &mut BenchmarkGroup<WallTime>) {
    let rt = rt();

    g.bench_function("unbounded", |b| {
        b.iter(|| {
            rt.block_on(async move {
                let (mut tx, mut rx) = mpsc::unbounded::<usize>();

                for i in 0..5000 {
                    tx.send(i).await.unwrap();
                }

                for _ in 0..5_000 {
                    let _ = rx.next().await;
                }
            })
        })
    });
}

fn contention_unbounded(g: &mut BenchmarkGroup<WallTime>) {
    let rt = rt();

    g.bench_function("unbounded", |b| {
        b.iter(|| {
            rt.block_on(async move {
                let (tx, mut rx) = mpsc::unbounded::<usize>();
                for _ in 0..5 {
                    let mut tx = tx.clone();
                    spawn(async move {
                        for i in 0..1000 {
                            tx.send(i).await.unwrap();
                        }
                    });
                }
                for _ in 0..1_000 * 5 {
                    let _ = rx.next().await;
                }
            })
        })
    });
}
