use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};
use std::thread;
use std::time::Duration;

use atomic_waker::AtomicWaker;
use easy_parallel::Parallel;
use nio_task::Task;

// Creates a future with event counters.
//
// Usage: `future!(f, get_waker, POLL, DROP)`
//
// The future `f` always sleeps for 200 ms and returns `Poll::Pending`.
// When it gets polled, `POLL` is incremented.
// When it gets dropped, `DROP` is incremented.
//
// Every time the future is run, it stores the waker into a global variable.
// This waker can be extracted using the `get_waker()` function.
macro_rules! future {
    ($name:pat, $get_waker:pat, $poll:ident, $drop:ident) => {
        static $poll: AtomicUsize = AtomicUsize::new(0);
        static $drop: AtomicUsize = AtomicUsize::new(0);
        static WAKER: AtomicWaker = AtomicWaker::new();

        let ($name, $get_waker) = {
            struct Fut(#[allow(dead_code)] Box<i32>);

            impl Future for Fut {
                type Output = ();

                fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                    WAKER.register(cx.waker());
                    $poll.fetch_add(1, Ordering::SeqCst);
                    thread::sleep(ms(400));
                    Poll::Pending
                }
            }

            impl Drop for Fut {
                fn drop(&mut self) {
                    $drop.fetch_add(1, Ordering::SeqCst);
                }
            }

            (Fut(Box::new(0)), || WAKER.take().unwrap())
        };
    };
}

// Creates a schedule function with event counters.
//
// Usage: `schedule!(s, chan, SCHED, DROP)`
//
// The schedule function `s` pushes the task into `chan`.
// When it gets invoked, `SCHED` is incremented.
// When it gets dropped, `DROP` is incremented.
//
// Receiver `chan` extracts the task when it is scheduled.
macro_rules! schedule {
    ($name:pat, $chan:pat, $sched:ident, $drop:ident) => {
        static $drop: AtomicUsize = AtomicUsize::new(0);
        static $sched: AtomicUsize = AtomicUsize::new(0);

        let ($name, $chan) = {
            let (s, r) = flume::unbounded();

            struct Guard(#[allow(dead_code)] Box<i32>);

            impl Drop for Guard {
                fn drop(&mut self) {
                    $drop.fetch_add(1, Ordering::SeqCst);
                }
            }

            let guard = Guard(Box::new(0));
            let sched = move |runnable: Task| {
                let _ = &guard;
                $sched.fetch_add(1, Ordering::SeqCst);
                s.send(runnable).unwrap();
            };

            (sched, r)
        };
    };
}

fn ms(ms: u64) -> Duration {
    Duration::from_millis(ms)
}

#[test]
fn wake_during_run() {
    future!(f, get_waker, POLL, DROP_F);
    schedule!(s, chan, SCHEDULE, DROP_S);
    let (task, _) = Task::new(f, s);

    task.poll();
    let waker = get_waker();
    waker.wake_by_ref();
    let task = chan.recv().unwrap();

    Parallel::new()
        .add(|| {
            task.poll();
            assert_eq!(POLL.load(Ordering::SeqCst), 2);
            assert_eq!(SCHEDULE.load(Ordering::SeqCst), 1);
            assert_eq!(DROP_F.load(Ordering::SeqCst), 0);
            assert_eq!(DROP_S.load(Ordering::SeqCst), 0);
            assert_eq!(chan.len(), 0);
        })
        .add(|| {
            thread::sleep(ms(200));

            waker.wake_by_ref();
            assert_eq!(POLL.load(Ordering::SeqCst), 2);
            assert_eq!(SCHEDULE.load(Ordering::SeqCst), 1);
            assert_eq!(DROP_F.load(Ordering::SeqCst), 0);
            assert_eq!(DROP_S.load(Ordering::SeqCst), 0);
            assert_eq!(chan.len(), 0);

            thread::sleep(ms(400));

            assert_eq!(POLL.load(Ordering::SeqCst), 2);
            assert_eq!(SCHEDULE.load(Ordering::SeqCst), 1);
            assert_eq!(DROP_F.load(Ordering::SeqCst), 0);
            assert_eq!(DROP_S.load(Ordering::SeqCst), 0);
            assert_eq!(chan.len(), 0);
        })
        .run();

    assert!(chan.is_empty());
    drop(get_waker());
}

#[test]
fn cancel_during_run() {
    future!(f, get_waker, POLL, DROP_F);
    schedule!(s, chan, SCHEDULE, DROP_S);
    let (task, join) = Task::new(f, s);

    task.poll();
    let waker = get_waker();
    waker.wake();
    let runnable = chan.recv().unwrap();

    Parallel::new()
        .add(|| {
            runnable.poll();
            drop(get_waker());
            assert_eq!(POLL.load(Ordering::SeqCst), 2);
            assert_eq!(SCHEDULE.load(Ordering::SeqCst), 1);
            assert_eq!(DROP_F.load(Ordering::SeqCst), 1);
            assert_eq!(DROP_S.load(Ordering::SeqCst), 1);
            assert_eq!(chan.len(), 0);
        })
        .add(|| {
            thread::sleep(ms(200));

            drop(join);
            assert_eq!(POLL.load(Ordering::SeqCst), 2);
            assert_eq!(SCHEDULE.load(Ordering::SeqCst), 1);
            assert_eq!(DROP_F.load(Ordering::SeqCst), 0);
            assert_eq!(DROP_S.load(Ordering::SeqCst), 0);
            assert_eq!(chan.len(), 0);

            thread::sleep(ms(400));

            assert_eq!(POLL.load(Ordering::SeqCst), 2);
            assert_eq!(SCHEDULE.load(Ordering::SeqCst), 1);
            assert_eq!(DROP_F.load(Ordering::SeqCst), 1);
            assert_eq!(DROP_S.load(Ordering::SeqCst), 1);
            assert_eq!(chan.len(), 0);
        })
        .run();
}

#[test]
fn wake_and_cancel_during_run() {
    future!(f, get_waker, POLL, DROP_F);
    schedule!(s, chan, SCHEDULE, DROP_S);
    let (task, join) = Task::new(f, s);

    task.poll();
    let waker = get_waker();
    waker.wake_by_ref();
    let task = chan.recv().unwrap();

    Parallel::new()
        .add(|| {
            task.poll();
            drop(get_waker());
            assert_eq!(POLL.load(Ordering::SeqCst), 2);
            assert_eq!(SCHEDULE.load(Ordering::SeqCst), 1);
            assert_eq!(DROP_F.load(Ordering::SeqCst), 1);
            assert_eq!(DROP_S.load(Ordering::SeqCst), 1);
            assert_eq!(chan.len(), 0);
        })
        .add(|| {
            thread::sleep(ms(200));

            waker.wake();
            assert_eq!(POLL.load(Ordering::SeqCst), 2);
            assert_eq!(SCHEDULE.load(Ordering::SeqCst), 1);
            assert_eq!(DROP_F.load(Ordering::SeqCst), 0);
            assert_eq!(DROP_S.load(Ordering::SeqCst), 0);
            assert_eq!(chan.len(), 0);

            drop(join);
            assert_eq!(POLL.load(Ordering::SeqCst), 2);
            assert_eq!(SCHEDULE.load(Ordering::SeqCst), 1);
            assert_eq!(DROP_F.load(Ordering::SeqCst), 0);
            assert_eq!(DROP_S.load(Ordering::SeqCst), 0);
            assert_eq!(chan.len(), 0);

            thread::sleep(ms(400));

            assert_eq!(POLL.load(Ordering::SeqCst), 2);
            assert_eq!(SCHEDULE.load(Ordering::SeqCst), 1);
            assert_eq!(DROP_F.load(Ordering::SeqCst), 1);
            assert_eq!(DROP_S.load(Ordering::SeqCst), 1);
            assert_eq!(chan.len(), 0);
        })
        .run();
}

#[test]
fn cancel_and_wake_during_run() {
    future!(f, get_waker, POLL, DROP_F);
    schedule!(s, chan, SCHEDULE, DROP_S);
    let (task, join) = Task::new(f, s);

    task.poll();
    let waker = get_waker();
    waker.wake_by_ref();
    let task = chan.recv().unwrap();

    Parallel::new()
        .add(|| {
            task.poll();
            drop(get_waker());
            assert_eq!(POLL.load(Ordering::SeqCst), 2);
            assert_eq!(SCHEDULE.load(Ordering::SeqCst), 1);
            assert_eq!(DROP_F.load(Ordering::SeqCst), 1);
            assert_eq!(DROP_S.load(Ordering::SeqCst), 1);
            assert_eq!(chan.len(), 0);
        })
        .add(|| {
            thread::sleep(ms(200));

            drop(join);
            assert_eq!(POLL.load(Ordering::SeqCst), 2);
            assert_eq!(SCHEDULE.load(Ordering::SeqCst), 1);
            assert_eq!(DROP_F.load(Ordering::SeqCst), 0);
            assert_eq!(DROP_S.load(Ordering::SeqCst), 0);
            assert_eq!(chan.len(), 0);

            waker.wake();
            assert_eq!(POLL.load(Ordering::SeqCst), 2);
            assert_eq!(SCHEDULE.load(Ordering::SeqCst), 1);
            assert_eq!(DROP_F.load(Ordering::SeqCst), 0);
            assert_eq!(DROP_S.load(Ordering::SeqCst), 0);
            assert_eq!(chan.len(), 0);

            thread::sleep(ms(400));

            assert_eq!(POLL.load(Ordering::SeqCst), 2);
            assert_eq!(SCHEDULE.load(Ordering::SeqCst), 1);
            assert_eq!(DROP_F.load(Ordering::SeqCst), 1);
            assert_eq!(DROP_S.load(Ordering::SeqCst), 1);
            assert_eq!(chan.len(), 0);
        })
        .run();
}

#[test]
fn drop_last_waker() {
    future!(f, get_waker, POLL, DROP_F);
    schedule!(s, chan, SCHEDULE, DROP_S);
    let (task, join) = Task::new(f, s);

    task.poll();
    let waker = get_waker();

    drop(join);
    assert_eq!(POLL.load(Ordering::SeqCst), 1);
    assert_eq!(SCHEDULE.load(Ordering::SeqCst), 0);
    assert_eq!(DROP_F.load(Ordering::SeqCst), 0);
    assert_eq!(DROP_S.load(Ordering::SeqCst), 0);
    assert_eq!(chan.len(), 0);

    drop(waker);
    assert_eq!(POLL.load(Ordering::SeqCst), 1);
    assert_eq!(SCHEDULE.load(Ordering::SeqCst), 0);
    assert_eq!(DROP_F.load(Ordering::SeqCst), 1);
    assert_eq!(DROP_S.load(Ordering::SeqCst), 1);
    assert_eq!(chan.len(), 0);
}

#[test]
fn cancel_last_task() {
    future!(f, get_waker, POLL, DROP_F);
    schedule!(s, chan, SCHEDULE, DROP_S);
    let (task, join) = Task::new(f, s);

    task.poll();
    drop(get_waker());
    assert_eq!(POLL.load(Ordering::SeqCst), 1);
    assert_eq!(SCHEDULE.load(Ordering::SeqCst), 0);
    assert_eq!(DROP_F.load(Ordering::SeqCst), 0);
    assert_eq!(DROP_S.load(Ordering::SeqCst), 0);
    assert_eq!(chan.len(), 0);

    drop(join);
    assert_eq!(POLL.load(Ordering::SeqCst), 1);
    assert_eq!(SCHEDULE.load(Ordering::SeqCst), 0);
    assert_eq!(DROP_F.load(Ordering::SeqCst), 1);
    assert_eq!(DROP_S.load(Ordering::SeqCst), 1);
    assert_eq!(chan.len(), 0);
}

#[test]
fn drop_last_task() {
    future!(f, get_waker, POLL, DROP_F);
    schedule!(s, chan, SCHEDULE, DROP_S);
    let (task, join) = Task::new(f, s);

    task.poll();
    drop(get_waker());
    assert_eq!(POLL.load(Ordering::SeqCst), 1);
    assert_eq!(SCHEDULE.load(Ordering::SeqCst), 0);
    assert_eq!(DROP_F.load(Ordering::SeqCst), 0);
    assert_eq!(DROP_S.load(Ordering::SeqCst), 0);
    assert_eq!(chan.len(), 0);

    drop(join);
    assert_eq!(POLL.load(Ordering::SeqCst), 1);
    assert_eq!(SCHEDULE.load(Ordering::SeqCst), 0);
    assert_eq!(DROP_F.load(Ordering::SeqCst), 1);
    assert_eq!(DROP_S.load(Ordering::SeqCst), 1);
    assert_eq!(chan.len(), 0);
}
