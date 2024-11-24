#![warn(rust_2018_idioms)]
#![cfg(feature = "full")]

use nio::time::timeout;
use tokio::sync::oneshot;
use tokio_test::*;

use std::time::Duration;

#[nio::test]
async fn simultaneous_deadline_future_completion() {
    // Create a future that is immediately ready
    let mut fut = task::spawn(timeout(ms(0), async {}));
    // Ready!
    assert_ready_ok!(fut.poll());
}

#[nio::test]
async fn future_and_deadline_in_future() {
    // Not yet complete
    let (tx, rx) = oneshot::channel();

    // Wrap it with a deadline
    let mut fut = task::spawn(timeout(ms(100), rx));

    assert_pending!(fut.poll());

    // Turn the timer, it runs for the elapsed time

    assert_pending!(fut.poll());

    // Complete the future
    tx.send(()).unwrap();
    assert!(fut.is_woken());

    assert_ready_ok!(fut.poll()).unwrap();
}

#[nio::test]
async fn future_and_timeout_in_future() {
    // Not yet complete
    let (tx, rx) = oneshot::channel();

    // Wrap it with a deadline
    let mut fut = task::spawn(timeout(ms(100), rx));

    // Ready!
    assert_pending!(fut.poll());

    // Turn the timer, it runs for the elapsed time
    assert_pending!(fut.poll());

    // Complete the future
    tx.send(()).unwrap();

    assert_ready_ok!(fut.poll()).unwrap();
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
