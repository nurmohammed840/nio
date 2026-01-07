use nio_rt::spawn_local;
use std::{
    future::{Future, poll_fn},
    pin::Pin,
    task::{Context, Poll},
};

async fn waker() -> std::task::Waker {
    poll_fn(|cx| Poll::Ready(cx.waker().clone())).await
}

#[nio_rt::test]
async fn wakers_are_different() {
    let w1 = waker().await;

    assert!(w1.will_wake(&waker().await));

    spawn_local(async move {
        let w2 = waker().await;

        spawn_local(async move {
            let w3 = waker().await;

            assert!(!w1.will_wake(&w2));
            assert!(!w1.will_wake(&w3));
            assert!(!w2.will_wake(&w3));
        })
        .await
        .unwrap();
    })
    .await
    .unwrap();
}

#[nio_rt::test]
async fn doesnt_poll_after_ready() {
    #[derive(Default)]
    struct Bomb {
        returned_ready: bool,
    }
    impl Future for Bomb {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.returned_ready {
                panic!("Future was polled again after returning Poll::Ready");
            } else {
                self.returned_ready = true;
                Poll::Ready(())
            }
        }
    }
    Bomb::default().await
}
