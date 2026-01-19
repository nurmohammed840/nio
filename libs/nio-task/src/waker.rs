use crate::thin_arc::ThinArc;
use std::{
    mem::ManuallyDrop,
    task::{RawWaker, RawWakerVTable, Waker},
};

pub trait ArcWaker {
    fn wake(this: ThinArc<Self>);
    fn wake_by_ref(this: &ThinArc<Self>);
}

pub fn waker_from<W: ArcWaker>(waker: ThinArc<W>) -> Waker {
    unsafe { Waker::from_raw(raw_waker(waker)) }
}

fn raw_waker<W: ArcWaker>(waker: ThinArc<W>) -> RawWaker {
    // Increment the reference count of the arc to clone it.
    //
    // The #[inline(always)] is to ensure that raw_waker and clone_waker are
    // always generated in the same code generation unit as one another, and
    // therefore that the structurally identical const-promoted RawWakerVTable
    // within both functions is deduplicated at LLVM IR code generation time.
    // This allows optimizing Waker::will_wake to a single pointer comparison of
    // the vtable pointers, rather than comparing all four function pointers
    // within the vtables.
    #[inline(always)]
    unsafe fn clone_waker<W: ArcWaker>(waker: *const ()) -> RawWaker {
        let this = unsafe { ManuallyDrop::new(ThinArc::from_raw(waker as *const W)) };
        this.header().state.inc_ref();

        RawWaker::new(
            waker,
            &RawWakerVTable::new(
                clone_waker::<W>,
                wake::<W>,
                wake_by_ref::<W>,
                drop_waker::<W>,
            ),
        )
    }

    // Wake by value, moving the Arc into the Wake::wake function
    unsafe fn wake<W: ArcWaker>(waker: *const ()) {
        let this = unsafe { ThinArc::from_raw(waker as *const W) };
        <W as ArcWaker>::wake(this);
    }

    // Wake by reference, wrap the waker in ManuallyDrop to avoid dropping it
    unsafe fn wake_by_ref<W: ArcWaker>(waker: *const ()) {
        let this = unsafe { ManuallyDrop::new(ThinArc::from_raw(waker as *const W)) };
        <W as ArcWaker>::wake_by_ref(&this);
    }

    // Decrement the reference count of the Arc on drop
    unsafe fn drop_waker<W: ArcWaker>(waker: *const ()) {
        drop(unsafe { ThinArc::from_raw(waker as *const W) });
    }

    RawWaker::new(
        ThinArc::into_raw(waker) as *const (),
        &RawWakerVTable::new(
            clone_waker::<W>,
            wake::<W>,
            wake_by_ref::<W>,
            drop_waker::<W>,
        ),
    )
}
