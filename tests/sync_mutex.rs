#![warn(rust_2018_idioms)]

#[cfg(all(target_family = "wasm", not(target_os = "wasi")))]
use wasm_bindgen_test::wasm_bindgen_test as test;
#[cfg(all(target_family = "wasm", not(target_os = "wasi")))]
use wasm_bindgen_test::wasm_bindgen_test as maybe_tokio_test;

#[cfg(not(all(target_family = "wasm", not(target_os = "wasi"))))]
use nio::test as maybe_tokio_test;

use tokio::sync::Mutex;
use tokio_test::task::spawn;
use tokio_test::{assert_pending, assert_ready};

use std::sync::Arc;

#[test]
fn straight_execution() {
    let l = Mutex::new(100);

    {
        let mut t = spawn(l.lock());
        let mut g = assert_ready!(t.poll());
        assert_eq!(&*g, &100);
        *g = 99;
    }
    {
        let mut t = spawn(l.lock());
        let mut g = assert_ready!(t.poll());
        assert_eq!(&*g, &99);
        *g = 98;
    }
    {
        let mut t = spawn(l.lock());
        let g = assert_ready!(t.poll());
        assert_eq!(&*g, &98);
    }
}

#[test]
fn readiness() {
    let l1 = Arc::new(Mutex::new(100));
    let l2 = Arc::clone(&l1);
    let mut t1 = spawn(l1.lock());
    let mut t2 = spawn(l2.lock());

    let g = assert_ready!(t1.poll());

    // We can't now acquire the lease since it's already held in g
    assert_pending!(t2.poll());

    // But once g unlocks, we can acquire it
    drop(g);
    assert!(t2.is_woken());
    let _t2 = assert_ready!(t2.poll());
}

/// This test is similar to `aborted_future_1` but this time the
/// aborted future is waiting for the lock.
#[nio::test]
#[cfg(feature = "full")]
async fn aborted_future_2() {
    use nio::time::timeout;
    use std::time::Duration;

    let m1: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    {
        // Lock mutex
        let _lock = m1.lock().await;
        {
            let m2 = m1.clone();
            // Try to lock mutex in a future that is aborted prematurely
            timeout(Duration::from_millis(1u64), async move {
                let _g = m2.lock().await;
            })
            .await
            .unwrap_err();
        }
    }
    // This should succeed as there is no lock left for the mutex.
    timeout(Duration::from_millis(1u64), async move {
        let _g = m1.lock().await;
    })
    .await
    .expect("Mutex is locked");
}

#[test]
fn try_lock() {
    let m: Mutex<usize> = Mutex::new(0);
    {
        let g1 = m.try_lock();
        assert!(g1.is_ok());
        let g2 = m.try_lock();
        assert!(g2.is_err());
    }
    let g3 = m.try_lock();
    assert!(g3.is_ok());
}

#[maybe_tokio_test]
async fn debug_format() {
    let s = "debug";
    let m = Mutex::new(s.to_string());
    assert_eq!(format!("{:?}", s), format!("{:?}", m.lock().await));
}

#[maybe_tokio_test]
async fn mutex_debug() {
    let s = "data";
    let m = Mutex::new(s.to_string());
    assert_eq!(format!("{:?}", m), r#"Mutex { data: "data" }"#);
    let _guard = m.lock().await;
    assert_eq!(format!("{:?}", m), r#"Mutex { data: <locked> }"#)
}
