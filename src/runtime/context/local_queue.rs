use super::LOCAL_QUEUE;
use crate::scheduler::TaskQueue;
use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
};

pub struct LocalQueue {
    pub queue: &'static UnsafeCell<TaskQueue>,
}

impl LocalQueue {
    /// ### Note
    ///
    /// This function uses [`Box::leak`] to create a static reference.
    /// Therefore, the caller must explicitly call [`LocalQueue::drop`]
    /// to release the memory.
    pub fn new(q: TaskQueue) -> Self {
        let queue: &_ = Box::leak(Box::new(UnsafeCell::new(q)));
        LOCAL_QUEUE.with(|q| unsafe { *q.get() = Some(LocalQueue { queue }) });
        Self { queue }
    }

    /// # Safety
    ///
    /// Caller must not obtain multiple references in different places on the call stack.
    ///
    /// For example, calling [LocalQueue::current] recursively or within nested closures
    /// can lead to undefined behavior!
    ///
    /// So Caller can't do this:
    ///
    /// ```ignore
    /// let mut queue = LocalQueue::new(queue);
    /// let q1 = queue.deref();
    /// unsafe {
    ///     LocalQueue::current(|q2| { // UB! })
    /// }
    /// ```
    ///
    /// Or this:
    ///
    /// ```rust, ignire
    /// unsafe {
    ///     LocalQueue::current(|q1| {
    ///         LocalQueue::current(|q2| {
    ///             // Undefined behavior!
    ///         });
    ///     });
    ///     // ===========================================
    ///     let cb = || LocalQueue::current(|q2| { ... });
    ///     let cb = LocalQueue::current(|q1| {
    ///         || { ... ; cb() }
    ///     });
    ///     cb(); // calling this function is UB!
    ///     // ===========================================
    ///     LocalQueue::current(|q1| {
    ///         cb(); // calling this function is also UB!
    ///         // ...
    ///     });
    /// }
    /// ```
    #[inline]
    pub unsafe fn current<F, R>(f: F) -> R
    where
        F: FnOnce(Option<&mut TaskQueue>) -> R,
    {
        LOCAL_QUEUE.with(|q| {
            let q = (*q.get()).as_ref();
            let q = q.map(|q| &mut *q.queue.get());
            f(q)
        })
    }

    pub fn drop(self) -> Box<TaskQueue> {
        unsafe { LocalQueue::current(|mut q| Box::from_raw(q.take().unwrap())) }
    }
}

impl Deref for LocalQueue {
    type Target = TaskQueue;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.queue.get() }
    }
}

impl DerefMut for LocalQueue {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.queue.get() }
    }
}
