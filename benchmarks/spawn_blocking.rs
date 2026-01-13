use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::thread;

use import::rt::*;

mod import {
    pub mod rt;
    // pub mod rt {
    //     pub mod tokio;
    //     pub use tokio::*;
    // }
}

/// Number of batches per task
const NUM_BATCHES: usize = 100;
/// Number of spawn_blocking calls per batch
const BATCH_SIZE: usize = 16;

fn spawn_blocking_concurrency(c: &mut Criterion) {
    let max_parallelism = thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1);

    let parallelism_levels: Vec<usize> = [1, 2, 4, 8, 16, 32, 64]
        .into_iter()
        .filter(|&n| n <= max_parallelism)
        .collect();

    let mut group = c.benchmark_group("spawn_blocking");
    for num_tasks in parallelism_levels {
        group.bench_with_input(
            BenchmarkId::new("concurrency", num_tasks),
            &num_tasks,
            |b, &num_tasks| {
                let rt = Runtime::multi();

                b.iter(|| {
                    rt.block_on(async move {
                        let mut tasks = Vec::with_capacity(num_tasks);

                        for _ in 0..num_tasks {
                            tasks.push(spawn(async {
                                for _ in 0..NUM_BATCHES {
                                    let mut handles = Vec::with_capacity(BATCH_SIZE);
                                    for _ in 0..BATCH_SIZE {
                                        handles.push(spawn_blocking(|| black_box(0)));
                                    }

                                    for handle in handles {
                                        assert_eq!(handle.await.unwrap(), 0);
                                    }
                                }
                            }));
                        }

                        for task in tasks {
                            task.await.unwrap();
                        }
                    });
                });
            },
        );
    }

    group.finish();
}

criterion_group!(spawn_blocking_benches, spawn_blocking_concurrency);
criterion_main!(spawn_blocking_benches);
