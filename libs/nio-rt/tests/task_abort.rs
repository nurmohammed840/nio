use std::{sync::Arc, thread, time::Duration};

use nio_rt::{sleep, spawn, spawn_local, test};

#[test]
async fn test_abort_without_panic_3157() {
    let handle = spawn_local(async move { sleep(Duration::new(100, 0)).await });

    // wait for task to sleep.
    sleep(Duration::from_millis(10)).await;

    handle.abort_handle().abort();
    let _ = handle.await;
}

/// Checks that a suspended task can be aborted inside of a current_thread
/// executor without panicking as reported in issue #3662:
#[test(worker_threads = 2)]
async fn test_abort_without_panic_3662() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct DropCheck(Arc<AtomicBool>);

    impl Drop for DropCheck {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    let drop_flag = Arc::new(AtomicBool::new(false));
    let drop_check = DropCheck(drop_flag.clone());

    let j = spawn(async move {
        // NB: just grab the drop check here so that it becomes part of the
        // task.
        let _drop_check = drop_check;
        std::future::pending::<()>().await;
    });

    let drop_flag2 = drop_flag.clone();

    let task = thread::spawn(move || {
        // This runs in a separate thread so it doesn't have immediate
        // thread-local access to the executor. It does however transition
        // the underlying task to be completed, which will cause it to be
        // dropped (but not in this thread).
        assert!(!drop_flag2.load(Ordering::SeqCst));
        j.abort_handle().abort();
        j
    })
    .join()
    .unwrap();

    let result = task.await;
    assert!(drop_flag.load(Ordering::SeqCst));
    assert!(result.unwrap_err().is_cancelled());

    // Note: We do the following to trigger a deferred task cleanup.
    //
    // The relevant piece of code you want to look at is in:
    // `Inner::block_on` of `scheduler/current_thread.rs`.
    //
    // We cause the cleanup to happen by having a poll return Pending once
    // so that the scheduler can go into the "auxiliary tasks" mode, at
    // which point the task is removed from the scheduler.
    let i = spawn(async move {
        nio_future::yield_now().await;
    });

    i.await.unwrap();
}

#[test]
async fn remote_abort() {
    struct DropCheck {
        created_on: thread::ThreadId,
        not_send: std::marker::PhantomData<*const ()>,
    }

    impl DropCheck {
        fn new() -> Self {
            Self {
                created_on: thread::current().id(),
                not_send: std::marker::PhantomData,
            }
        }
    }
    impl Drop for DropCheck {
        fn drop(&mut self) {
            if thread::current().id() != self.created_on {
                panic!("non-Send value dropped in another thread!");
            }
        }
    }

    let check = DropCheck::new();
    let jh = spawn_local(async move {
        std::future::pending::<()>().await;
        drop(check);
    });

    let jh_abort = jh.abort_handle();
    let jh2 = thread::spawn(move || {
        thread::sleep(Duration::from_millis(10));
        jh_abort.abort();
    });

    assert!(jh.await.is_err());
    jh2.join().unwrap();
}

/// Checks that a suspended task can be aborted even if the `JoinHandle` is immediately dropped.
/// issue #3964: <https://github.com/tokio-rs/tokio/issues/3964>.
#[test]
async fn test_abort_wakes_task_3964() {
    let notify_dropped = Arc::new(());
    let weak_notify_dropped = Arc::downgrade(&notify_dropped);

    let handle = spawn_local(async move {
        // Make sure the Arc is moved into the task
        let _notify_dropped = notify_dropped;
        sleep(Duration::new(100, 0)).await
    });

    // wait for task to sleep.
    sleep(Duration::from_millis(10)).await;

    handle.abort_handle().abort();
    drop(handle);

    // wait for task to abort.
    sleep(Duration::from_millis(10)).await;

    // Check that the Arc has been dropped.
    assert!(weak_notify_dropped.upgrade().is_none());
}

#[cfg(panic = "unwind")]
struct PanicOnDrop;

#[cfg(panic = "unwind")]
impl Drop for PanicOnDrop {
    fn drop(&mut self) {
        panic!("Well what did you expect would happen...");
    }
}

/// Checks that aborting a task whose destructor panics does not allow the
/// panic to escape the task.
#[test]
#[cfg(panic = "unwind")]
async fn test_abort_task_that_panics_on_drop_contained() {
    let handle = spawn_local(async move {
        // Make sure the Arc is moved into the task
        let _panic_dropped = PanicOnDrop;
        sleep(Duration::new(100, 0)).await
    });

    // wait for task to sleep.
    sleep(Duration::from_millis(10)).await;

    handle.abort_handle().abort();
    drop(handle);

    // wait for task to abort.
    sleep(Duration::from_millis(10)).await;
}

/// Checks that aborting a task whose destructor panics has the expected result.
#[test]
#[cfg(panic = "unwind")]
async fn test_abort_task_that_panics_on_drop_returned() {
    let handle = spawn_local(async move {
        // Make sure the Arc is moved into the task
        let _panic_dropped = PanicOnDrop;
        sleep(Duration::new(100, 0)).await
    });

    // wait for task to sleep.
    sleep(Duration::from_millis(10)).await;

    handle.abort_handle().abort();
    assert!(handle.await.unwrap_err().is_panic());
}

// It's not clear where these tests belong. This was the place suggested by @Darksonn:
// https://github.com/tokio-rs/tokio/pull/6753#issuecomment-2271434176
/// Checks that a `JoinError` with a panic payload prints the expected text.
#[test(worker_threads = 2)]
#[cfg(panic = "unwind")]
async fn test_join_error_display() {
    // `String` payload
    let join_err = spawn(async move {
        let value = 1234;
        panic!("Format-args payload: {value}")
    })
    .await
    .unwrap_err();

    // We can't assert the full output because the task ID can change.
    let join_err_str = join_err.to_string();

    assert!(
        join_err_str.starts_with("task ")
            && join_err_str.ends_with(" panicked with message \"Format-args payload: 1234\""),
        "Unexpected join_err_str {join_err_str:?}"
    );

    // `&'static str` payload
    let join_err = spawn(async move { panic!("Const payload") })
        .await
        .unwrap_err();

    let join_err_str = join_err.to_string();

    assert!(
        join_err_str.starts_with("task ")
            && join_err_str.ends_with(" panicked with message \"Const payload\""),
        "Unexpected join_err_str {join_err_str:?}"
    );

    // Non-string payload
    let join_err = spawn(async move { std::panic::panic_any(1234i32) })
        .await
        .unwrap_err();

    let join_err_str = join_err.to_string();

    assert!(
        join_err_str.starts_with("task ") && join_err_str.ends_with(" panicked"),
        "Unexpected join_err_str {join_err_str:?}"
    );
}
