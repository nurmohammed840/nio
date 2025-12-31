mod sleep;

use std::{
    cell::Cell,
    cmp,
    collections::BTreeMap,
    fmt, mem,
    rc::Rc,
    time::{Duration, Instant},
};

use crate::local_waker::LocalWaker;
use sleep::Sleep;

#[derive(Default)]
pub struct Timers {
    entries: BTreeMap<TimerEntry, ()>,
}

#[derive(Eq)]
struct TimerEntry {
    timer: *const Timer,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    None,
    Notified,
}

pub struct Timer {
    deadline: Cell<Instant>,
    state: Cell<State>,
    waker: LocalWaker,
}

impl Timer {
    pub fn new(deadline: Instant) -> Self {
        Self {
            deadline: Cell::new(deadline),
            state: Cell::new(State::None),
            waker: LocalWaker::new(),
        }
    }
}

impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool {
        self.deadline() == other.deadline() && self.timer == other.timer
    }
}

impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        match self.deadline().cmp(&other.deadline()) {
            cmp::Ordering::Equal => self.timer.cmp(&other.timer),
            ord => ord,
        }
    }
}

impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl TimerEntry {
    fn deadline(&self) -> Instant {
        unsafe { (*self.timer).deadline.get() }
    }

    fn timer_as_ref(&self) -> &Timer {
        unsafe { &*self.timer }
    }

    fn timer(self) -> Rc<Timer> {
        unsafe { Rc::from_raw(self.timer) }
    }
}

impl Timers {
    pub fn new() -> Timers {
        Self::default()
    }

    fn remove(&mut self, timer: &Timer) {
        if let Some((entry, _)) = self.entries.remove_entry(&TimerEntry { timer }) {
            drop(entry.timer());
        }
    }

    fn reset(&mut self, timer: &Timer, new_deadline: Instant) {
        if let Some((entry, _)) = self.entries.remove_entry(&TimerEntry { timer }) {
            entry.timer_as_ref().deadline.set(new_deadline);
            self.entries.insert(entry, ());
        }
    }

    fn sleep(&mut self, dur: Duration) -> Sleep {
        let deadline = Instant::now() + dur;
        let timer = Rc::new(Timer::new(deadline));
        self.entries.insert(
            TimerEntry {
                timer: Rc::into_raw(timer.clone()),
            },
            (),
        );
        Sleep { timer }
    }

    pub fn next_timeout(&self, since: Instant) -> Option<Duration> {
        self.entries
            .first_key_value()?
            .0
            .deadline()
            .checked_duration_since(since)
    }

    pub fn fetch(&mut self, upto: Instant) -> Option<Self> {
        let timer = Timer::new(upto + Duration::from_millis(1));
        let right = self.entries.split_off(&TimerEntry { timer: &timer });
        let left = mem::replace(&mut self.entries, right);
        if left.is_empty() {
            return None;
        }
        Some(Self { entries: left })
    }

    pub fn notify_all(self) {
        for (entry, _) in self.entries {
            let timer = entry.timer();
            timer.state.set(State::Notified);
            timer.waker.wake();
        }
    }
}

impl fmt::Debug for Timer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Timer")
            .field("deadline", &self.deadline)
            .field("state", &self.state.get())
            .finish()
    }
}

impl fmt::Debug for Timers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut map = f.debug_list();
        for (entry, _) in &self.entries {
            map.entry(unsafe { &(*entry.timer) });
        }
        map.finish()
    }
}
