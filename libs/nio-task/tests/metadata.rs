use flume::unbounded;
use nio_task::{Status, Task};
use smol::future;

#[test]
fn metadata_use_case() {
    // Each future has a counter that is incremented every time it is scheduled.
    let (sender, receiver) = unbounded::<Task<u32>>();

    let schedule = move |mut task: Task<u32>| {
        *task.metadata_mut() += 1;
        println!("{}", task.metadata());
        sender.send(task).ok();
    };

    async fn my_future() {
        for _ in 0..5 {
            future::yield_now().await;
        }
    }

    let make_task = || {
        // SAFETY: We are spawning a non-'static future, so we need to use the unsafe API.
        // The borrowed variables, in this case the metadata, are guaranteed to outlive the runnable.
        let (runnable, task) = Task::new_with(0, my_future(), schedule.clone());
        runnable.schedule();
        task
    };

    // Make tasks.
    let t1 = make_task();
    let t2 = make_task();

    // Run the tasks.
    while let Ok(task) = receiver.try_recv() {
        match task.poll() {
            Status::Yielded(task) => {
                task.schedule();
            }
            Status::Pending => {}
            Status::Complete(metadata) => {
                // We schedule 6 times: 1 initially + 5 for yield.
                assert_eq!(metadata.get(), &6);
            }
        }
    }

    // Unwrap the tasks.
    smol::future::block_on(async move {
        let _ = t1.await;
        let _ = t2.await;
    });
}
