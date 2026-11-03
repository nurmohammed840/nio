use core::{
    cell::UnsafeCell,
    fmt,
    marker::PhantomData,
    task::{Context, Waker},
};
use std::slice;

#[derive(Default)]
pub struct LocalWakerSet {
    wakers: UnsafeCell<Wakers>,
    // mark as !Send type.
    _phantom: PhantomData<*const ()>,
}

impl LocalWakerSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, cx: &mut Context<'_>) {
        let waker = cx.waker();

        let seen = unsafe { (*self.wakers.get()).iter() }.any(|prev| waker.will_wake(prev));
        if seen {
            return;
        }

        let waker = waker.clone();
        unsafe { (*self.wakers.get()).add(waker) };
    }

    #[inline]
    pub fn wake(&self) {
        match self.take() {
            Wakers::One(None) => {}
            Wakers::One(Some(waker)) => {
                waker.wake();
            }
            Wakers::Many(mut wakers) => {
                for waker in wakers.drain(..) {
                    waker.wake();
                }
                // Reuse same allocation
                unsafe {
                    *self.wakers.get() = Wakers::Many(wakers);
                }
            }
        }
    }

    pub fn take(&self) -> Wakers {
        unsafe { (*self.wakers.get()).take() }
    }
}

pub enum Wakers {
    One(Option<Waker>),
    Many(Vec<Waker>),
}

impl Default for Wakers {
    fn default() -> Self {
        Wakers::One(None)
    }
}

impl Wakers {
    #[inline]
    fn iter(&self) -> Iter<'_> {
        match self {
            Wakers::Many(thin_vec) => Iter::Many(thin_vec.iter()),
            Wakers::One(waker) => Iter::Once(waker.as_ref()),
        }
    }

    fn add(&mut self, waker: Waker) {
        match self {
            Wakers::Many(vals) => vals.push(waker),
            Wakers::One(slot) => match slot.take() {
                None => *slot = Some(waker),
                Some(prev) => *self = Wakers::Many(vec![prev, waker]),
            },
        }
    }

    fn take(&mut self) -> Wakers {
        std::mem::take(self)
    }
}

enum Iter<'a> {
    Many(slice::Iter<'a, Waker>),
    Once(Option<&'a Waker>),
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Waker;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Iter::Many(vals) => vals.next(),
            Iter::Once(val) => val.take(),
        }
    }
}

impl fmt::Debug for LocalWakerSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LocalWakerSet")
    }
}
