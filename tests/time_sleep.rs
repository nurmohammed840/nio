#![warn(rust_2018_idioms)]
#![cfg(feature = "full")]
// #![cfg(not(miri))] // Too slow on Miri.

use nio::time;
use std::time::Duration;

#[nio::test]
async fn immediate_sleep() {
    // Ready!
    time::sleep(ms(0)).await;
}

#[nio::test]
async fn short_sleeps() {
    for _ in 0..1000 {
        nio::time::sleep(std::time::Duration::from_millis(0)).await;
    }
}

#[nio::test]
#[cfg_attr(miri, ignore)]
async fn issue_5183() {
    let big = std::time::Duration::from_secs(u64::MAX / 64);
    // This is a workaround since awaiting sleep(big) will never finish.
    #[rustfmt::skip]
    tokio::select! {
	biased;
        _ = nio::time::sleep(big) => {}
        _ = nio::time::sleep(std::time::Duration::from_nanos(1)) => {}
    }
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

#[nio::test]
#[cfg_attr(miri, ignore)]
async fn drop_from_wake() {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::task::Context;

    let panicked = Arc::new(AtomicBool::new(false));
    let list: Arc<Mutex<Vec<Pin<Box<nio::time::Sleep>>>>> = Arc::new(Mutex::new(Vec::new()));

    let arc_wake = Arc::new(DropWaker(panicked.clone(), list.clone()));
    let arc_wake = futures::task::waker(arc_wake);

    {
        let mut lock = list.lock().unwrap();

        for _ in 0..100 {
            let mut timer = Box::pin(nio::time::sleep(Duration::from_millis(10)));

            let _ = timer.as_mut().poll(&mut Context::from_waker(&arc_wake));

            lock.push(timer);
        }
    }

    nio::time::sleep(Duration::from_millis(11)).await;

    assert!(
        !panicked.load(Ordering::SeqCst),
        "panicked when dropping timers"
    );

    #[derive(Clone)]
    struct DropWaker(Arc<AtomicBool>, Arc<Mutex<Vec<Pin<Box<nio::time::Sleep>>>>>);

    impl futures::task::ArcWake for DropWaker {
        fn wake_by_ref(arc_self: &Arc<Self>) {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                *arc_self.1.lock().expect("panic in lock") = Vec::new()
            }));

            if result.is_err() {
                arc_self.0.store(true, Ordering::SeqCst);
            }
        }
    }
}
