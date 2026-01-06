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
        match unsafe { &*self.waker.get() } {
            Some(old) if waker.will_wake(old) => return,
            _ => {}
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
        let has_waker = unsafe { (*self.waker.get()).is_some() };
        write!(f, "LocalWaker: {has_waker}")
    }
}
