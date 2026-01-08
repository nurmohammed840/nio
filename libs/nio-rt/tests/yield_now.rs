use std::future::poll_fn;
use std::task::Poll;

#[nio_rt::test]
async fn yield_now() {
    let mut yielded = false;

    let yield_now = poll_fn(|cx| {
        if yielded {
            return Poll::Ready(());
        }
        yielded = true;
        cx.waker().wake_by_ref();
        Poll::Pending
    });

    yield_now.await;

    println!("Done");
}
