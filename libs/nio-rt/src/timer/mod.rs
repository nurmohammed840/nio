pub mod interval;
pub mod sleep;
pub mod timeout;

use crate::local_waker::LocalWaker;
use sleep::Sleep;

use std::{
    cell::Cell,
    cmp,
    collections::BTreeMap,
    fmt, mem,
    rc::Rc,
    time::{Duration, Instant},
};

type Entries = BTreeMap<TimerEntry, ()>;
pub struct Clock(Cell<Instant>);

pub struct Timers {
    entries: Entries,
    pub clock: Clock,
}

#[derive(Eq)]
struct TimerEntry {
    timer: *const Timer,
}

pub struct Timer {
    deadline: Cell<Instant>,
    notified: Cell<bool>,
    waker: LocalWaker,
}

impl Timer {
    pub fn new(deadline: Instant) -> Self {
        Self {
            deadline: Cell::new(deadline),
            notified: Cell::new(false),
            waker: LocalWaker::new(),
        }
    }
}

impl PartialEq for TimerEntry {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.timer == other.timer
    }
}

impl PartialOrd for TimerEntry {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(TimerEntry::cmp(self, other))
    }
}

impl Ord for TimerEntry {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        match self.deadline().cmp(&other.deadline()) {
            cmp::Ordering::Equal => self.timer.cmp(&other.timer),
            ord => ord,
        }
    }
}

impl TimerEntry {
    #[inline]
    fn deadline(&self) -> Instant {
        unsafe { (*self.timer).deadline.get() }
    }

    #[inline]
    fn set_deadline(&self, deadline: Instant) {
        unsafe { (*self.timer).deadline.set(deadline) }
    }

    #[inline]
    fn timer(self) -> Rc<Timer> {
        unsafe { Rc::from_raw(self.timer) }
    }
}

impl Timers {
    pub fn new() -> Timers {
        Timers {
            entries: Entries::new(),
            clock: Clock::new(),
        }
    }

    fn remove(&mut self, timer: &Timer) {
        if let Some((entry, _)) = self.entries.remove_entry(&TimerEntry { timer }) {
            drop(entry.timer());
        }
    }

    fn reset_at(&mut self, timer: &Timer, deadline: Instant) {
        if let Some((entry, _)) = self.entries.remove_entry(&TimerEntry { timer }) {
            entry.set_deadline(deadline);
            self.entries.insert(entry, ());
        }
    }

    fn sleep_at(&mut self, deadline: Instant) -> Sleep {
        let timer = Rc::new(Timer::new(deadline));
        self.insert_entry(timer.clone());
        Sleep { timer }
    }

    fn insert_entry(&mut self, timer: Rc<Timer>) {
        self.entries.insert(
            TimerEntry {
                timer: Rc::into_raw(timer),
            },
            (),
        );
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
        let right = self.entries.split_off(&TimerEntry { timer });
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
            let timer = entry.timer();
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
            map.entry(unsafe { &(*entry.timer) });
        }
        map.finish()
    }
}
