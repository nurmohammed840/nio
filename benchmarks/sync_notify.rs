use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion,
};
use std::sync::atomic::{AtomicUsize, Ordering};

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

criterion_main!(notify_waiters_simple);
criterion_group!(
    notify_waiters_simple,
    bench_notify_one,
    bench_notify_waiters
);

fn rt() -> Runtime {
    Runtime::new(6)
}

fn bench_notify_one(c: &mut Criterion) {
    let mut group = c.benchmark_group("notify_one");
    notify_one::<10>(&mut group);
    notify_one::<50>(&mut group);
    notify_one::<100>(&mut group);
    notify_one::<200>(&mut group);
    notify_one::<500>(&mut group);
    group.finish();
}

fn bench_notify_waiters(c: &mut Criterion) {
    let mut group = c.benchmark_group("notify_waiters");
    notify_waiters::<10>(&mut group);
    notify_waiters::<50>(&mut group);
    notify_waiters::<100>(&mut group);
    notify_waiters::<200>(&mut group);
    notify_waiters::<500>(&mut group);
    group.finish();
}

fn notify_one<const N_WAITERS: usize>(g: &mut BenchmarkGroup<WallTime>) {
    let rt = rt();

    static NOTIFY: Notify = Notify::const_new();
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    for _ in 0..N_WAITERS {
        rt.spawn({
            async move {
                loop {
                    NOTIFY.notified().await;
                    COUNTER.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
    }

    const N_ITERS: usize = 500;
    g.bench_function(N_WAITERS.to_string(), |b| {
        b.iter(|| {
            COUNTER.store(0, Ordering::Relaxed);
            loop {
                NOTIFY.notify_one();
                if COUNTER.load(Ordering::Relaxed) >= N_ITERS {
                    break;
                }
            }
        })
    });
}

fn notify_waiters<const N_WAITERS: usize>(g: &mut BenchmarkGroup<WallTime>) {
    let rt = rt();
    static NOTIFY: Notify = Notify::const_new();
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    for _ in 0..N_WAITERS {
        rt.spawn({
            async move {
                loop {
                    NOTIFY.notified().await;
                    COUNTER.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
    }

    const N_ITERS: usize = 500;
    g.bench_function(N_WAITERS.to_string(), |b| {
        b.iter(|| {
            COUNTER.store(0, Ordering::Relaxed);
            loop {
                NOTIFY.notify_waiters();
                if COUNTER.load(Ordering::Relaxed) >= N_ITERS {
                    break;
                }
            }
        })
    });
}
