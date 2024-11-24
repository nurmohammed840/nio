use std::{
    cell::UnsafeCell,
    collections::BinaryHeap,
    future::Future,
    hash::{Hash, Hasher},
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};

use crate::runtime;

const EMPTY: usize = 0;
const REGISTERED: usize = 1;
const ELAPSED: usize = 2;
const CANCELED: usize = 3;

// TODO: Reduce Size
pub struct Timer {
    state: AtomicUsize,
    waker: UnsafeCell<Option<Waker>>,
    deadline: Instant,
}

unsafe impl Sync for Timer {}

impl Eq for Timer {}
impl PartialEq for Timer {
    fn eq(&self, other: &Self) -> bool {
        self.deadline == other.deadline
    }
}
impl PartialOrd for Timer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(other.deadline.cmp(&self.deadline))
    }
}
impl Ord for Timer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.deadline.cmp(&self.deadline)
    }
}
impl Hash for Timer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.deadline.hash(state);
    }
}

#[derive(Default)]
pub struct Timers {
    queue: BinaryHeap<Arc<Timer>>,
}

impl Timers {
    pub fn add(&mut self, timer: Arc<Timer>) {
        self.queue.push(timer);
    }

    pub fn peek(&self) -> Option<&Arc<Timer>> {
        self.queue.peek()
    }

    pub fn next_timeout(&self) -> Option<Duration> {
        self.queue
            .peek()?
            .deadline
            .checked_duration_since(Instant::now())
    }

    /// ## Time Complexity
    ///
    /// - **O(1)** if there are no timers to dispatch.
    /// - **O(T . log(N)*)** where `T` is the number of timers that have reached their deadline
    ///   and need to be dispatched, and `N` is the total number of active timers.
    ///
    ///   For example, if there are 1,000 active timers and 10 of them have reached their deadline,
    ///   In worse case scenario, the time complexity would be `O(10 . log(1000)*) = O(30*)`
    pub fn process(&mut self) {
        let now = Instant::now();
        while self.queue.peek().is_some_and(|timer| now >= timer.deadline) {
            let timer = unsafe { self.queue.pop().unwrap_unchecked() };
            if timer.state.swap(ELAPSED, Ordering::AcqRel) == REGISTERED {
                unsafe {
                    (*timer.waker.get()).take().unwrap().wake();
                }
            }
        }
    }
}

pub struct Sleep {
    timer: Arc<Timer>,
}

pub fn sleep(dur: Duration) -> Sleep {
    let timer = Arc::new(Timer {
        state: AtomicUsize::new(EMPTY),
        waker: UnsafeCell::new(None),
        deadline: Instant::now() + dur,
    });

    runtime::context::with(|ctx| {
        let reactor = ctx.reactor();
        let should_wake = {
            let mut timers = reactor.shared.timers.lock().unwrap();
            let should_wake = timers
                .peek()
                .is_none_or(|next_timer| timer.deadline <= next_timer.deadline);

            timers.add(timer.clone());
            should_wake
        };
        if should_wake {
            reactor.wake();
        }
    });

    Sleep { timer }
}

impl Future for Sleep {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let state = self.timer.state.load(Ordering::Acquire);
        if state == ELAPSED {
            return Poll::Ready(());
        }
        if state == EMPTY {
            unsafe {
                *self.timer.waker.get() = Some(cx.waker().clone());
            }
            if self.timer.state.swap(REGISTERED, Ordering::Release) == ELAPSED {
                return Poll::Ready(());
            }
        }
        Poll::Pending
    }
}

impl Drop for Sleep {
    fn drop(&mut self) {
        self.timer.state.store(CANCELED, Ordering::Release);
    }
}
