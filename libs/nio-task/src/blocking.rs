use crate::raw::{Header, RawTask, RawTaskVTable, Fut};
use crate::waker::NOOP_WAKER;
use crate::{Id, JoinError, JoinHandle};

use std::fmt;
use std::task::{Poll, Waker};
use std::{cell::UnsafeCell, sync::Arc};

pub struct BlockingTask {
    raw_task: RawTask,
}

impl BlockingTask {
    pub fn new<F, T>(f: F) -> (BlockingTask, JoinHandle<T>)
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let raw_task = Arc::new(BlockingRawTask {
            header: Header::new(),
            func: UnsafeCell::new(Fut::Running(f)),
        });
        let join = JoinHandle::new(raw_task.clone());
        (BlockingTask { raw_task }, join)
    }

    pub fn run(self) {
        unsafe { self.raw_task.poll(&NOOP_WAKER) };
    }

    #[inline]
    pub fn id(&self) -> Id {
        Id::new(&self.raw_task)
    }
}

pub struct BlockingRawTask<F, T> {
    header: Header,
    func: UnsafeCell<Fut<F, T>>,
}

unsafe impl<F: Send, T> Sync for BlockingRawTask<F, T> {}

impl<F, T> RawTaskVTable for BlockingRawTask<F, T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    fn header(&self) -> &Header {
        &self.header
    }

    fn waker(self: Arc<Self>) -> Waker {
        NOOP_WAKER
    }

    unsafe fn drop_task_or_output(&self) {
        *self.func.get() = Fut::Droped;
    }

    /// Panicking is acceptable here, as `BlockingTask` is only execute within the thread pool
    unsafe fn poll(&self, _: &Waker) -> bool {
        let output = match (*self.func.get()).take() {
            Fut::Running(func) => func(),
            _ => unreachable!(),
        };
        (*self.func.get()).set_output(output);
        if !self.header.transition_to_complete_and_notify_output_if_intrested() {
            unsafe { self.drop_task_or_output() };
        }
        false
    }

    unsafe fn read_output(&self, dst: *mut (), waker: &Waker) {
        if self.header.can_read_output(waker) {
            *(dst as *mut _) = Poll::Ready(self.take_output());
        }
    }

    unsafe fn abort_task(self: Arc<Self>) {}

    unsafe fn schedule(self: Arc<Self>) {}
}

impl<F, T> BlockingRawTask<F, T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    unsafe fn take_output(&self) -> Result<F::Output, JoinError> {
        match (*self.func.get()).take() {
            Fut::Result(output) => output,
            _ => panic!("JoinHandle polled after completion"),
        }
    }
}

impl fmt::Debug for BlockingTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockingTask")
            .field("id", &self.id())
            .field("state", &self.raw_task.header().state.load())
            .finish()
    }
}
