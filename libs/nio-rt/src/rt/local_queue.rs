use nio_task::Task;
use std::{cell::UnsafeCell, collections::VecDeque, rc::Rc};

#[derive(Clone)]
pub struct LocalQueue {
    inner: Rc<UnsafeCell<VecDeque<Task>>>,
}

impl LocalQueue {
    pub fn new(cap: usize) -> Self {
        Self {
            inner: Rc::new(UnsafeCell::new(VecDeque::with_capacity(cap))),
        }
    }
    // Safety: the caller must ensure that there are no references alive when this is called.
    pub unsafe fn get_mut(&self) -> &mut VecDeque<Task> {
        unsafe { &mut *self.inner.get() }
    }
}
