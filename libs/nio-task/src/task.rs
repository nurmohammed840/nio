use std::{
    cell::UnsafeCell,
    panic::{AssertUnwindSafe, catch_unwind},
    pin::Pin,
    task::{Context, Poll, Waker},
};

use crate::{
    JoinError, Scheduler, Task,
    raw::{Fut, PollStatus, RawTask, RawTaskHeader, RawTaskVTable},
    thin_arc::ThinArc,
    waker::ArcWaker,
};

pub struct RawTaskInner<F: Future, S: Scheduler<M>, M> {
    pub future: UnsafeCell<Fut<F, F::Output>>,
    pub meta: UnsafeCell<M>,
    pub scheduler: S,
}

impl<F, S, M> RawTaskVTable for RawTaskHeader<RawTaskInner<F, S, M>>
where
    M: 'static,
    F: Future + 'static,
    S: Scheduler<M>,
{
    #[inline]
    fn waker(&self, raw: RawTask) -> Waker {
        // SAFETY: stable rust doesn't support `self: ThinArc<Self>`, so we manually convert it.
        let this: ThinArc<Self> = unsafe { ThinArc::concrete(raw) };
        crate::waker::waker_from(this)
    }

    unsafe fn metadata(&self) -> *mut () {
        self.data.meta.get().cast()
    }

    /// State transitions:
    ///
    /// ```markdown
    /// NOTIFIED -> RUNNING -> ( SLEEP? -> NOTIFIED -> RUNNING )* -> COMPLETE?
    /// ```
    unsafe fn poll(&self, waker: &Waker) -> PollStatus {
        let is_cancelled = self.header.transition_to_running_and_check_if_cancelled();

        let has_output = catch_unwind(AssertUnwindSafe(|| {
            let result = if is_cancelled {
                Err(JoinError::cancelled())
            } else {
                let poll_result = unsafe {
                    let fut = match &mut *self.data.future.get() {
                        Fut::Future(fut) => Pin::new_unchecked(fut),
                        _ => unreachable!(),
                    };
                    // Polling may panic, but we catch it in outer layer.
                    fut.poll(&mut Context::from_waker(waker))
                };
                match poll_result {
                    Poll::Ready(val) => Ok(val),
                    Poll::Pending => return false,
                }
            };
            // Droping `Fut::Future` may also panic, but we catch it in outer layer
            unsafe {
                (*self.data.future.get()).set_output(result);
            }
            true
        }));

        match has_output {
            Ok(false) => return self.header.transition_to_sleep(),
            Ok(true) => {}
            Err(err) => unsafe { (*self.data.future.get()).set_output(Err(JoinError::panic(err))) },
        }
        if !self
            .header
            .transition_to_complete_and_notify_output_if_intrested()
        {
            // Receiver is not interested in the output, So we can drop it.
            // Droping `Fut::Output` may panic
            let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
                (*self.data.future.get()).drop()
            }));
        }
        PollStatus::Complete
    }

    unsafe fn schedule(&self, raw: RawTask) {
        self.data.scheduler.schedule(Task::from_raw(raw));
    }

    unsafe fn drop_task(&self) {
        // TODO: Also drop task metadata.

        let may_panic = catch_unwind(AssertUnwindSafe(|| {
            (*self.data.future.get()).set_output(Err(JoinError::cancelled()));
        }));
        if let Err(panic) = may_panic {
            (*self.data.future.get()).set_output(Err(JoinError::panic(panic)));
        }
        if !self
            .header
            .transition_to_complete_and_notify_output_if_intrested()
        {
            unsafe { (*self.data.future.get()).drop() }
        }
    }

    unsafe fn abort_task(&self, raw: RawTask) {
        if self.header.transition_to_abort() {
            self.schedule(raw)
        }
    }

    unsafe fn read_output(&self, dst: *mut (), waker: &Waker) {
        if self.header.can_read_output_or_notify_when_readable(waker) {
            *(dst as *mut _) = Poll::Ready((*self.data.future.get()).take_output());
        }
    }

    unsafe fn drop_join_handler(&self) {
        let is_task_complete = self.header.state.unset_waker_and_interested();
        if is_task_complete {
            // If the task is complete then waker is droped by the executor.
            // We just only need to drop the output.
            let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
                (*self.data.future.get()).drop();
            }));
        } else {
            *self.header.join_waker.get() = None;
        }
    }
}

impl<F, S, M> RawTaskHeader<RawTaskInner<F, S, M>>
where
    M: 'static,
    F: Future + 'static,
    S: Scheduler<M>,
{
    unsafe fn schedule_by_ref(this: &ThinArc<Self>) {
        this.data
            .scheduler
            .schedule(Task::from_raw(ThinArc::erase(this.clone())));
    }
}

impl<F, S, M> ArcWaker for RawTaskHeader<RawTaskInner<F, S, M>>
where
    M: 'static,
    F: Future + 'static,
    S: Scheduler<M>,
{
    fn wake(this: ThinArc<Self>) {
        unsafe {
            if this.header.transition_to_notified() {
                Self::schedule_by_ref(&this);
            }
        }
    }

    fn wake_by_ref(this: &ThinArc<Self>) {
        unsafe {
            if this.header.transition_to_notified() {
                Self::schedule_by_ref(this);
            }
        }
    }
}
