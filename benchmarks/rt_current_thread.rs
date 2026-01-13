use criterion::{criterion_group, criterion_main, Criterion};

use import::rt::*;

mod import {
    pub mod rt;
    // pub mod rt {
    //     pub mod tokio;
    //     pub use tokio::*;
    // }
}

const NUM_SPAWN: usize = 1_000;

fn rt_curr_spawn_many_local(c: &mut Criterion) {
    let rt = Runtime::new(1);

    c.bench_function("spawn_many_local", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut handles = Vec::with_capacity(NUM_SPAWN);

                for _ in 0..NUM_SPAWN {
                    handles.push(spawn(async move {}));
                }

                for handle in handles {
                    handle.await.unwrap();
                }
            });
        })
    });
}

fn rt_curr_spawn_many_remote_busy(c: &mut Criterion) {
    let rt = Runtime::new(1);
    
    rt.spawn(async {
        fn iter() {
            spawn(async { iter() });
        }
        iter()
    });

    c.bench_function("spawn_many_remote_busy", |b| {
        b.iter(|| {
            rt.block_on(async move {
                let mut handles = Vec::with_capacity(NUM_SPAWN);

                for _ in 0..NUM_SPAWN {
                    handles.push(spawn(async {}));
                }

                for handle in handles {
                    handle.await.unwrap();
                }
            });
        })
    });
}

criterion_group!(
    rt_curr_scheduler,
    rt_curr_spawn_many_local,
    rt_curr_spawn_many_remote_busy
);

criterion_main!(rt_curr_scheduler);
