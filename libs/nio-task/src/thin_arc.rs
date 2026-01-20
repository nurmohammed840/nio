#![allow(warnings)]
use crate::raw::{Header, RawTaskHeader, RawTaskVTable};
use std::{marker::PhantomData, mem::ManuallyDrop, ptr::NonNull};

pub struct ThinArc<Data: ?Sized> {
    ptr: NonNull<Data>,
    phantom: PhantomData<Data>,
}

impl<T: ?Sized> Unpin for ThinArc<T> {}
unsafe impl<T: ?Sized + Sync + Send> Send for ThinArc<T> {}
unsafe impl<T: ?Sized + Sync + Send> Sync for ThinArc<T> {}

impl ThinArc<dyn RawTaskVTable> {
    #[inline]
    pub fn new<Data>(this: Box<RawTaskHeader<Data>>) -> (Self, Self)
    where
        RawTaskHeader<Data>: RawTaskVTable + 'static,
    {
        let ptr: NonNull<dyn RawTaskVTable + 'static> = NonNull::from(Box::leak(this));
        unsafe { (ThinArc::from_inner(ptr), ThinArc::from_inner(ptr)) }
    }
}

impl<Data: ?Sized> ThinArc<Data> {
    #[inline]
    pub fn header(&self) -> &Header {
        unsafe { &*self.ptr.as_ptr().cast() }
    }

    #[inline]
    unsafe fn from_inner(ptr: NonNull<Data>) -> Self {
        Self {
            ptr,
            phantom: PhantomData,
        }
    }

    pub fn as_ptr(&self) -> *const Data {
        NonNull::as_ptr(self.ptr)
    }

    pub fn into_raw(this: Self) -> *const Data {
        let this = ManuallyDrop::new(this);
        NonNull::as_ptr(this.ptr)
    }

    pub fn clone_without_ref_inc(&self) -> Self {
        unsafe { ThinArc::from_inner(self.ptr) }
    }

    pub unsafe fn from_raw(ptr: *const Data) -> Self {
        ThinArc::from_inner(NonNull::new_unchecked(ptr as *mut Data))
    }

    // Non-inlined part of `drop`.
    #[inline(never)]
    unsafe fn drop_slow(&mut self) {
        drop(Box::from_raw(self.ptr.as_ptr()));
    }
}

impl<T: RawTaskVTable + 'static> ThinArc<T> {
    /// We need to manually impl `erase`
    /// as we can't impl `CoerceUnsized` on stable rust
    ///
    /// https://doc.rust-lang.org/std/ops/trait.CoerceUnsized.html
    pub fn erase(this: ThinArc<T>) -> ThinArc<dyn RawTaskVTable> {
        let this = ManuallyDrop::new(this);
        unsafe { ThinArc::from_inner(this.ptr) }
    }

    // same as: https://doc.rust-lang.org/std/sync/struct.Arc.html#method.downcast_unchecked
    pub unsafe fn concrete(this: ThinArc<dyn RawTaskVTable>) -> ThinArc<T> {
        let this = ManuallyDrop::new(this);
        unsafe { ThinArc::from_inner(this.ptr.cast()) }
    }
}

impl<T: ?Sized> std::ops::Deref for ThinArc<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }
}

impl<Data: ?Sized> Clone for ThinArc<Data> {
    #[inline]
    fn clone(&self) -> Self {
        self.header().state.inc_ref();
        unsafe { ThinArc::from_inner(self.ptr) }
    }
}

impl<Data: ?Sized> Drop for ThinArc<Data> {
    #[inline]
    fn drop(&mut self) {
        if self.header().state.ref_dec() {
            unsafe { self.drop_slow() };
        }
    }
}
