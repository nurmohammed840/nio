use std::task::{RawWaker, RawWakerVTable, Waker};

pub(crate) const NOOP_WAKER: Waker = unsafe { Waker::from_raw(raw_waker(&())) };
const NOOP_VTABLE: RawWakerVTable = RawWakerVTable::new(raw_waker, noop, noop, noop);
const fn noop(_: *const ()) {}
const fn raw_waker(data: *const ()) -> RawWaker {
    RawWaker::new(data, &NOOP_VTABLE)
}
