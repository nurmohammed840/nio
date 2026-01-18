pub mod interval;
pub mod sleep;
pub mod timeout;

use crate::local_waker::LocalWaker;
use sleep::Sleep;

use std::{
    cell::Cell,
    cmp,
    collections::BTreeMap,
    fmt,
    marker::PhantomData,
    mem::{self, ManuallyDrop},
    ptr::NonNull,
    time::{Duration, Instant},
};

type Entries = BTreeMap<RcTimer, ()>;
pub struct Clock(Cell<Instant>);

pub struct Timers {
    entries: Entries,
    pub clock: Clock,
}

#[derive(Eq)]
pub struct RcTimer {
    ptr: NonNull<Timer>,
    phantom: PhantomData<Timer>,
}

impl PartialEq for RcTimer {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl PartialOrd for RcTimer {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(RcTimer::cmp(self, other))
    }
}

impl Ord for RcTimer {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        match self.deadline().cmp(&other.deadline()) {
            cmp::Ordering::Equal => self.ptr.cmp(&other.ptr),
            ord => ord,
        }
    }
}

impl RcTimer {
    #[inline]
    fn deadline(&self) -> Instant {
        self.as_ref().deadline.get()
    }

    #[inline]
    fn set_deadline(&self, deadline: Instant) {
        self.as_ref().deadline.set(deadline)
    }
}

impl RcTimer {
    #[inline]
    fn as_ref(&self) -> &Timer {
        // This unsafety is ok because while this Rc is alive we're guaranteed
        // that the inner pointer is valid.
        unsafe { self.ptr.as_ref() }
    }

    #[inline]
    fn from_inner(ptr: NonNull<Timer>) -> RcTimer {
        RcTimer {
            ptr,
            phantom: PhantomData,
        }
    }

    #[inline]
    fn clone(&self) -> RcTimer {
        self.as_ref().rc.update(|rc| {
            assert_eq!(rc, 1);
            2
        });
        RcTimer::from_inner(self.ptr)
    }

    #[inline]
    fn create(deadline: Instant) -> (RcTimer, RcTimer) {
        let ptr = NonNull::from(Box::leak(Box::new(Timer::new(deadline))));
        (RcTimer::from_inner(ptr), RcTimer::from_inner(ptr))
    }

    #[inline(never)]
    unsafe fn drop_slow(&mut self) {
        unsafe {
            drop(Box::from_raw(self.ptr.as_ptr()));
        };
    }
}

impl Drop for RcTimer {
    #[inline]
    fn drop(&mut self) {
        self.as_ref().rc.update(|rc| rc - 1);
        if self.as_ref().rc.get() == 0 {
            unsafe { self.drop_slow() };
        }
    }
}

struct Timer {
    rc: Cell<u8>,
    deadline: Cell<Instant>,
    notified: Cell<bool>,
    waker: LocalWaker,
}

impl Timer {
    pub fn new(deadline: Instant) -> Self {
        Self {
            rc: Cell::new(2),
            deadline: Cell::new(deadline),
            notified: Cell::new(false),
            waker: LocalWaker::new(),
        }
    }
}

impl Timers {
    pub fn new() -> Timers {
        Timers {
            entries: Entries::new(),
            clock: Clock::new(),
        }
    }

    fn remove(&mut self, timer: &RcTimer) {
        if let Some((entry, _)) = self.entries.remove_entry(timer) {
            drop(entry);
        }
    }

    fn reset_at(&mut self, timer: &RcTimer, deadline: Instant) {
        if let Some((entry, _)) = self.entries.remove_entry(timer) {
            entry.set_deadline(deadline);
            self.entries.insert(entry, ());
        }
    }

    fn sleep_at(&mut self, deadline: Instant) -> Sleep {
        let (timer, other) = RcTimer::create(deadline);
        self.entries.insert(other, ());
        Sleep { timer }
    }

    fn insert_entry(&mut self, timer: RcTimer) {
        self.entries.insert(timer, ());
    }

    pub fn next_timeout(&self, since: Instant) -> Option<Duration> {
        if self.entries.is_empty() {
            return None;
        }
        self.entries
            .first_key_value()?
            .0
            .deadline()
            .checked_duration_since(since)
    }

    pub fn fetch(&mut self, upto: Instant) -> Elapsed {
        let timer = &Timer::new(upto);
        let entry = ManuallyDrop::new(RcTimer::from_inner(timer.into()));

        let right = self.entries.split_off(&entry);
        let left = mem::replace(&mut self.entries, right);
        Elapsed { entries: left }
    }
}

pub struct Elapsed {
    entries: Entries,
}

impl Elapsed {
    /// [`crate::LocalContext::add_task_to_local_queue`]
    pub fn notify_all(self) {
        for (entry, _) in self.entries {
            let timer = entry.as_ref();
            if timer.rc.get() == 1 {
                // Other half is droped
                return;
            }
            timer.notified.set(true);
            timer.waker.wake();
        }
    }
}

impl Clock {
    fn new() -> Self {
        Self(Cell::new(Instant::now()))
    }

    #[inline]
    pub fn current(&self) -> Instant {
        self.0.get()
    }

    pub fn now(&self) -> Instant {
        let time = Instant::now();
        self.0.set(time);
        time
    }
}

impl fmt::Debug for Timer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let deadline = self.deadline.get().checked_duration_since(Instant::now());
        f.debug_struct("Timer")
            .field("deadline", &deadline)
            .field("state", &self.notified.get())
            .finish()
    }
}

impl fmt::Debug for Timers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut map = f.debug_list();
        for entry in self.entries.keys() {
            map.entry(entry.as_ref());
        }
        map.finish()
    }
}
