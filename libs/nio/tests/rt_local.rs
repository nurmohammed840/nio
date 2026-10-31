use nio::{RuntimeBuilder, spawn, spawn_local, test};
use nio_future::yield_now;

mod support {
    pub mod futures;
}
use support::futures::sync::oneshot;

#[test]
async fn test_spawn_local_in_runtime() {
    let (tx, rx) = oneshot::channel();

    spawn_local(async {
        yield_now().await;
        tx.send(5).unwrap();
    });

    assert_eq!(rx.await.unwrap(), 5);
}

#[test]
async fn test_spawn_from_handle() {
    let (tx, rx) = oneshot::channel();
    spawn(async {
        yield_now().await;
        tx.send(5).unwrap();
    });
    assert_eq!(rx.await.unwrap(), 5);
}

#[test]
async fn test_spawn_local_on_runtime_object() {
    let (tx, rx) = tokio::sync::oneshot::channel();
    spawn_local(async {
        yield_now().await;
        tx.send(5).unwrap();
    });
    assert_eq!(rx.await.unwrap(), 5);
}

#[::core::prelude::v1::test]
#[cfg_attr(target_family = "wasm", ignore)] // threads not supported
fn test_spawn_from_guard_other_thread() {
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let rt = RuntimeBuilder::new().rt().unwrap();
        let handle = rt.context();

        tx.send(handle).unwrap();
    });

    let handle = rx.recv().unwrap();

    handle.enter();
    spawn(async {});
}
