use criterion::{criterion_group, criterion_main, Criterion};
use futures::channel::oneshot;
use std::hint::black_box;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use import::rt::*;
use import::task::yield_now;

mod import {
    pub mod rt;
    pub mod task;
    // pub mod rt {
    //     pub mod tokio;
    //     pub use tokio::*;
    // }
}

const NUM_WORKERS: usize = 4;
const NUM_SPAWN: usize = 10_000;
const STALL_DUR: Duration = Duration::from_micros(10);

fn rt_multi_spawn_many_local(c: &mut Criterion) {
    let rt = Runtime::new(NUM_WORKERS);
    let ctx = rt.handle();

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

fn rt_multi_spawn_many_remote(c: &mut Criterion) {
    let rt = Runtime::new(NUM_WORKERS);
    c.bench_function("spawn_many_remote", |b| {
        b.iter(|| {
            let c = rt.block_on(async {
                const NUM_TASK: usize = NUM_SPAWN / NUM_WORKERS;
                let mut workers = Vec::with_capacity(NUM_WORKERS);
                for _ in 0..NUM_WORKERS {
                    workers.push(spawn_pinned(|| async {
                        let mut tasks = Vec::with_capacity(NUM_TASK);
                        for _ in 0..NUM_TASK {
                            tasks.push(spawn(async { black_box(1) }));
                        }
                        let mut c = 0;
                        for task in tasks {
                            c += task.await.unwrap();
                        }
                        c
                    }));
                }
                let mut c = 0;
                for task in workers {
                    c += task.await.unwrap();
                }
                c
            });
            assert_eq!(c, NUM_SPAWN);
        })
    });
}

fn rt_multi_spawn_many_remote_idle(c: &mut Criterion) {
    let rt = Runtime::new(NUM_WORKERS);

    c.bench_function("spawn_many_remote_idle", |b| {
        b.iter(|| {
            let mut handles = Vec::with_capacity(NUM_SPAWN);
            for _ in 0..NUM_SPAWN {
                handles.push(rt.spawn(async {}));
            }
            rt.block_on(async {
                for handle in handles {
                    handle.await.unwrap();
                }
            });
        })
    });
}

// The runtime is busy with tasks that consume CPU time and yield. Yielding is a
// lower notification priority than spawning / regular notification.
fn rt_multi_spawn_many_remote_busy1(c: &mut Criterion) {
    let rt = Runtime::new(NUM_WORKERS);
    let rt_handle = rt.handle();

    static FLAG: AtomicBool = AtomicBool::new(true);

    // Spawn some tasks to keep the runtimes busy
    for _ in 0..(2 * NUM_WORKERS) {
        rt.spawn(async move {
            while FLAG.load(Relaxed) {
                yield_now().await;
                stall();
            }
        });
    }

    c.bench_function("spawn_many_remote_busy1", |b| {
        b.iter(|| {
            let mut handles = Vec::with_capacity(NUM_SPAWN);

            for _ in 0..NUM_SPAWN {
                handles.push(rt_handle.spawn(async {}));
            }

            rt.block_on(async {
                for handle in handles {
                    handle.await.unwrap();
                }
            });
        })
    });

    FLAG.store(false, Relaxed);
}

// The runtime is busy with tasks that consume CPU time and spawn new high-CPU
// tasks. Spawning goes via a higher notification priority than yielding.
fn rt_multi_spawn_many_remote_busy2(c: &mut Criterion) {
    const NUM_SPAWN: usize = 1_000;

    let rt = Runtime::new(NUM_WORKERS);
    let rt_handle = rt.handle();
    static FLAG: AtomicBool = AtomicBool::new(true);

    // Spawn some tasks to keep the runtimes busy
    for _ in 0..(NUM_WORKERS) {
        fn iter() {
            spawn(async {
                if FLAG.load(Relaxed) {
                    stall();
                    iter();
                }
            });
        }
        rt.spawn(async { iter() });
    }

    c.bench_function("spawn_many_remote_busy2", |b| {
        b.iter(|| {
            let mut handles = Vec::with_capacity(NUM_SPAWN);
            for _ in 0..NUM_SPAWN {
                handles.push(rt_handle.spawn(async {}));
            }
            rt.block_on(async move {
                for handle in handles {
                    handle.await.unwrap();
                }
            });
        })
    });

    FLAG.store(false, Relaxed);
}

fn rt_multi_ping_pong(c: &mut Criterion) {
    const NUM_PINGS: usize = 1_000;

    let rt = Runtime::new(NUM_WORKERS);

    let (done_tx, done_rx) = mpsc::sync_channel(1000);
    static REM: AtomicUsize = AtomicUsize::new(0);

    c.bench_function("ping_pong", |b| {
        b.iter(|| {
            REM.store(NUM_PINGS, Relaxed);
            let done_tx = done_tx.clone();

            rt.spawn(async move {
                for _ in 0..NUM_PINGS {
                    let done_tx = done_tx.clone();

                    spawn(async move {
                        let (tx1, rx1) = oneshot::channel();
                        let (tx2, rx2) = oneshot::channel();

                        spawn(async move {
                            rx1.await.unwrap();
                            tx2.send(()).unwrap();
                        });

                        tx1.send(()).unwrap();
                        rx2.await.unwrap();

                        if 1 == REM.fetch_sub(1, Relaxed) {
                            done_tx.send(()).unwrap();
                        }
                    });
                }
            });
            done_rx.recv().unwrap();
        })
    });
}

fn rt_multi_yield_many(c: &mut Criterion) {
    const NUM_YIELD: usize = 1_000;
    const TASKS: usize = 200;

    let rt = Runtime::new(NUM_WORKERS);

    c.bench_function("yield_many", |b| {
        let (tx, rx) = mpsc::sync_channel(TASKS);

        b.iter(|| {
            for _ in 0..TASKS {
                let tx = tx.clone();

                rt.spawn(async move {
                    for _ in 0..NUM_YIELD {
                        yield_now().await;
                    }

                    tx.send(()).unwrap();
                });
            }

            for _ in 0..TASKS {
                rx.recv().unwrap();
            }
        })
    });
}

fn rt_multi_chained_spawn(c: &mut Criterion) {
    const ITER: usize = 1_000;

    let rt = Runtime::new(NUM_WORKERS);

    fn iter(done_tx: mpsc::SyncSender<()>, n: usize) {
        if n == 0 {
            done_tx.send(()).unwrap();
        } else {
            spawn(async move {
                iter(done_tx, n - 1);
            });
        }
    }

    c.bench_function("chained_spawn", |b| {
        let (done_tx, done_rx) = mpsc::sync_channel(1000);

        b.iter(|| {
            let done_tx = done_tx.clone();

            rt.spawn(async move {
                iter(done_tx, ITER);
            });

            done_rx.recv().unwrap();
        })
    });
}

fn stall() {
    let now = Instant::now();
    while now.elapsed() < STALL_DUR {
        thread::yield_now();
    }
}

criterion_group!(
    rt_multi_scheduler,
    rt_multi_spawn_many_local,
    rt_multi_spawn_many_remote,
    rt_multi_spawn_many_remote_idle,
    rt_multi_spawn_many_remote_busy1,
    rt_multi_spawn_many_remote_busy2,
    rt_multi_ping_pong,
    rt_multi_yield_many,
    rt_multi_chained_spawn,
);

criterion_main!(rt_multi_scheduler);
