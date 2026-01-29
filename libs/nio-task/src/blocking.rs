use crate::raw::{Fut, Header, PollStatus, RawTask, RawTaskHeader, RawTaskVTable};
use crate::thin_arc::ThinArc;
use crate::{JoinError, JoinHandle, TaskId};

use std::cell::UnsafeCell;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::task::{Poll, Waker};

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
        let (raw, join) = unsafe {
            ThinArc::new(Box::new(RawTaskHeader {
                header: Header::new(),
                data: BlockingRawTask {
                    func: UnsafeCell::new(Fut::Future(f)),
                },
            }))
        };
        (BlockingTask { raw }, JoinHandle::new(join))
    }

    pub fn run(self) {
        unsafe { self.raw.poll(Waker::noop()) };
    }

    #[inline]
    pub fn id(&self) -> TaskId {
        TaskId::new(&self.raw)
    }
}

pub struct BlockingRawTask<F, T> {
    func: UnsafeCell<Fut<F, T>>,
}

impl<F, T> RawTaskVTable for RawTaskHeader<BlockingRawTask<F, T>>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    fn waker(&self, _: RawTask) -> Waker {
        unreachable!()
    }

    unsafe fn metadata(&self) -> *mut () {
        std::ptr::null_mut()
    }

    unsafe fn poll(&self, _: &Waker) -> PollStatus {
        let maybe_panicked = catch_unwind(AssertUnwindSafe(|| {
            let output = match (*self.data.func.get()).take() {
                Fut::Future(func) => func(), // Fn call may panic
                _ => unreachable!(),
            };
            // Droping Fn closure may also panic.
            (*self.data.func.get()).set_output(Ok(output));
        }));

        if let Err(err) = maybe_panicked {
            (*self.data.func.get()).set_output(Err(JoinError::panic(err)));
        }

        if !self
            .header
            .transition_to_complete_and_notify_output_if_intrested()
        {
            // Receiver is not interested in the output, So we can drop it.
            // Panicking is acceptable here, as `BlockingTask` is only execute within the thread pool
            unsafe { (*self.data.func.get()).drop() };
        }
        PollStatus::Complete
    }

    unsafe fn read_output(&self, dst: *mut (), waker: &Waker) {
        if self.header.can_read_output_or_notify_when_readable(waker) {
            *(dst as *mut _) = Poll::Ready((*self.data.func.get()).take_output());
        }
    }

    unsafe fn drop_output_from_join_handler(&self) {
        (*self.data.func.get()).drop();
    }

    unsafe fn schedule(&self, _: RawTask) {}
    unsafe fn drop_task(&self) {}
}

impl fmt::Debug for BlockingTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockingTask")
            .field("id", &self.id())
            .field("state", &self.raw.header().state.load())
            .finish()
    }
}
