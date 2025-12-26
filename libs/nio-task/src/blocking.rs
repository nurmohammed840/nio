use crate::raw::{Fut, Header, PollStatus, RawTask, RawTaskVTable};
use crate::waker::NOOP_WAKER;
use crate::{Id, JoinHandle};

use std::panic::AssertUnwindSafe;
use std::task::{Poll, Waker};
use std::{cell::UnsafeCell, sync::Arc};
use std::{fmt, panic};

pub struct BlockingTask {
    raw: RawTask,
}

unsafe impl Send for BlockingTask {}
unsafe impl Sync for BlockingTask {}

impl BlockingTask {
    pub fn new<F, T>(f: F) -> (BlockingTask, JoinHandle<T>)
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let raw = Arc::new(BlockingRawTask {
            header: Header::new(),
            func: UnsafeCell::new(Fut::Future(f)),
        });
        let join = JoinHandle::new(raw.clone());
        (BlockingTask { raw }, join)
    }

    pub fn run(self) {
        unsafe { self.raw.poll(&NOOP_WAKER) };
    }

    #[inline]
    pub fn id(&self) -> Id {
        Id::new(&self.raw)
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

    unsafe fn metadata(&self) -> *mut () {
        std::ptr::null_mut()
    }

    /// Panicking is acceptable here, as `BlockingTask` is only execute within the thread pool
    unsafe fn poll(&self, _: &Waker) -> PollStatus {
        let output = match (*self.func.get()).take() {
            Fut::Future(func) => func(),
            _ => unreachable!(),
        };
        (*self.func.get()).set_output(Ok(output));
        if !self
            .header
            .transition_to_complete_and_notify_output_if_intrested()
        {
            unsafe {
                (*self.func.get()).drop();
            };
        }
        PollStatus::Complete
    }

    unsafe fn read_output(&self, dst: *mut (), waker: &Waker) {
        if self.header.can_read_output_or_notify_when_readable(waker) {
            *(dst as *mut _) = Poll::Ready((*self.func.get()).take_output());
        }
    }

    unsafe fn drop_join_handler(&self) {
        let is_task_complete = self.header.state.unset_waker_and_interested();
        if is_task_complete {
            // If the task is complete then waker is droped by the executor.
            // We just need to drop the output
            let _ = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
                (*self.func.get()).drop();
            }));
        } else {
            *self.header.join_waker.get() = None;
        }
    }

    unsafe fn abort_task(self: Arc<Self>) {}
    unsafe fn schedule(self: Arc<Self>) {}
}

impl fmt::Debug for BlockingTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockingTask")
            .field("id", &self.id())
            .field("state", &self.raw.header().state.load())
            .finish()
    }
}
