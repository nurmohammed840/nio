use std::{
    future::Future,
    mem,
    pin::{Pin, pin},
    sync::{Arc, Condvar, Mutex},
    task::{Context, Poll, Wake, Waker},
};

pub async fn yield_now() {
    /// Yield implementation
    struct YieldNow {
        yielded: bool,
    }
    impl Future for YieldNow {
        type Output = ();
        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            if self.yielded {
                return Poll::Ready(());
            }
            self.yielded = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
    YieldNow { yielded: false }.await;
}

pub fn block_on<Fut>(fut: Fut) -> Fut::Output
where
    Fut: Future,
{
    let mut fut = pin!(fut);

    let signal = Arc::new(Signal::default());
    let waker = Waker::from(signal.clone());
    let mut cx = Context::from_waker(&waker);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(val) => return val,
            Poll::Pending => signal.wait_for_wakeup(),
        }
    }
}

const RUNNING: u8 = 0;
const NOTIFIED: u8 = 1;
const SLEEP: u8 = 2;

#[derive(Default)]
struct Signal {
    state: Mutex<u8>,
    signal: Condvar,
}

impl Signal {
    /// State transitions:
    ///
    /// ```text
    /// RUNNING -> (SLEEP? -> NOTIFIED -> RUNNING)* -> Complete?
    /// ```
    fn wait_for_wakeup(&self) {
        let mut state = self.state.lock().unwrap();
        if *state == NOTIFIED {
            *state = RUNNING;
        } else {
            *state = SLEEP;
            'spurious_wakeups: loop {
                state = self.signal.wait(state).unwrap();
                if *state == NOTIFIED {
                    *state = RUNNING;
                    break 'spurious_wakeups;
                }
            }
        }
    }
}

impl Wake for Signal {
    fn wake_by_ref(self: &Arc<Self>) {
        let state = {
            let mut state = self.state.lock().unwrap();
            mem::replace(&mut *state, NOTIFIED)
        };
        if state == SLEEP {
            self.signal.notify_one();
        }
    }

    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
}
