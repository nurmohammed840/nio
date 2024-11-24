use std::{
    future::Future,
    mem,
    pin::pin,
    sync::{Arc, Condvar, Mutex},
    task::{Context, Poll, Wake, Waker},
};

const ACTIVE: u8 = 0;
const WAKE: u8 = 1;
const SLEEP: u8 = 2;

pub fn block_on<Fut>(fut: Fut) -> Fut::Output
where
    Fut: Future,
{
    let mut fut = pin!(fut);

    let waker = Arc::new(ThreadWaker::default());
    let inner = Waker::from(waker.clone());
    let mut cx = Context::from_waker(&inner);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(val) => return val,
            Poll::Pending => waker.sleep(),
        }
    }
}

#[derive(Default)]
struct ThreadWaker {
    state: Mutex<u8>,
    signal: Condvar,
}

impl ThreadWaker {
    fn sleep(&self) {
        let mut state = self.state.lock().unwrap();
        if *state == WAKE {
            *state = ACTIVE;
        } else {
            *state = SLEEP;
            'spurious_wakeups: loop {
                state = self.signal.wait(state).unwrap();
                if *state == WAKE {
                    *state = ACTIVE;
                    break 'spurious_wakeups;
                }
            }
        }
    }
}

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        let old_state = {
            let mut state = self.state.lock().unwrap();
            mem::replace(&mut *state, WAKE)
        };
        if old_state == SLEEP {
            self.signal.notify_one();
        }
    }
}
