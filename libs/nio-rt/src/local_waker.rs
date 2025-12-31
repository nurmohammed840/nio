use std::{
    cell::UnsafeCell,
    fmt,
    marker::PhantomData,
    task::{Context, Waker},
};

#[derive(Default)]
pub struct LocalWaker {
    waker: UnsafeCell<Option<Waker>>,
    // mark LocalWaker as a !Send type.
    _phantom: PhantomData<*const ()>,
}

impl LocalWaker {
    /// Creates a new, empty `LocalWaker`.
    pub fn new() -> LocalWaker {
        LocalWaker::default()
    }

    pub fn register(&self, cx: &mut Context<'_>) {
        let waker = cx.waker();
        if let Some(prev) = unsafe { &*self.waker.get() } {
            if waker.will_wake(prev) {
                return;
            }
        }
        unsafe { *self.waker.get() = Some(waker.clone()) }
    }

    #[inline]
    pub fn wake(&self) {
        if let Some(waker) = self.take() {
            waker.wake();
        }
    }

    #[inline]
    pub fn take(&self) -> Option<Waker> {
        unsafe { (*self.waker.get()).take() }
    }
}

impl fmt::Debug for LocalWaker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LocalWaker")
    }
}
