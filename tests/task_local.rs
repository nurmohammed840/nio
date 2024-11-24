#![cfg(all(feature = "full", not(target_os = "wasi")))] // Wasi doesn't support threads

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::sync::oneshot;
use tokio::task_local;

#[nio::test]
async fn local() {
    task_local! {
        static REQ_ID: u32;
        pub static FOO: bool;
    }

    let j1 = nio::spawn(REQ_ID.scope(1, async move {
        assert_eq!(REQ_ID.get(), 1);
        assert_eq!(REQ_ID.get(), 1);
    }));

    let j2 = nio::spawn(REQ_ID.scope(2, async move {
        REQ_ID.with(|v| {
            assert_eq!(REQ_ID.get(), 2);
            assert_eq!(*v, 2);
        });

        nio::time::sleep(std::time::Duration::from_millis(10)).await;

        assert_eq!(REQ_ID.get(), 2);
    }));

    let j3 = nio::spawn(FOO.scope(true, async move {
        assert!(FOO.get());
    }));

    j1.await.unwrap();
    j2.await.unwrap();
    j3.await.unwrap();
}

#[nio::test]
async fn task_local_available_on_completion_drop() {
    task_local! {
        static KEY: u32;
    }

    struct MyFuture {
        tx: Option<oneshot::Sender<u32>>,
    }
    impl Future for MyFuture {
        type Output = ();

        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
            Poll::Ready(())
        }
    }
    impl Drop for MyFuture {
        fn drop(&mut self) {
            let _ = self.tx.take().unwrap().send(KEY.get());
        }
    }

    let (tx, rx) = oneshot::channel();

    let h = nio::spawn(KEY.scope(42, MyFuture { tx: Some(tx) }));

    assert_eq!(rx.await.unwrap(), 42);
    h.await.unwrap();
}

#[nio::test]
async fn take_value() {
    task_local! {
        static KEY: u32
    }
    let fut = KEY.scope(1, async {});
    let mut pinned = Box::pin(fut);
    assert_eq!(pinned.as_mut().take_value(), Some(1));
    assert_eq!(pinned.as_mut().take_value(), None);
}

#[nio::test]
async fn poll_after_take_value_should_fail() {
    task_local! {
        static KEY: u32
    }
    let fut = KEY.scope(1, async {
        let result = KEY.try_with(|_| {});
        // The task local value no longer exists.
        assert!(result.is_err());
    });
    let mut fut = Box::pin(fut);
    fut.as_mut().take_value();

    // Poll the future after `take_value` has been called
    fut.await;
}
