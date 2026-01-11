use std::{
    fmt,
    sync::atomic::{AtomicU64, Ordering},
};

const SHARED_COUNTER_BIT_SIZE: u8 = 32;
const SHARED_COUNTER_MASK: u64 = (1 << SHARED_COUNTER_BIT_SIZE) - 1;
const SHARED_COUNTER: u64 = 1;

const LOCAL_COUNTER: u64 = 1 << SHARED_COUNTER_BIT_SIZE;

pub struct TaskCounter {
    counter: AtomicU64,
}

#[derive(Clone, Copy)]
pub struct Counter(u64);

impl Counter {
    #[inline]
    pub fn local(self) -> u64 {
        self.0 >> SHARED_COUNTER_BIT_SIZE
    }

    #[inline]
    pub fn shared(self) -> u64 {
        self.0 & SHARED_COUNTER_MASK
    }

    #[inline]
    pub fn shared_queue_has_data(self) -> bool {
        self.shared() > 0
    }

    #[inline]
    pub fn total(self) -> u64 {
        self.local() + self.shared()
    }
}

impl TaskCounter {
    pub fn new() -> Self {
        Self {
            counter: AtomicU64::new(0),
        }
    }

    pub fn increase_local(&self) -> Counter {
        Counter(self.counter.fetch_add(LOCAL_COUNTER, Ordering::AcqRel))
    }

    pub fn decrease_local(&self) -> Counter {
        let old = Counter(self.counter.fetch_sub(LOCAL_COUNTER, Ordering::AcqRel));
        debug_assert!(old.local() > 0);
        old
    }

    /// Only this function is allowed to call from other thread.
    pub fn increase_shared(&self) {
        self.counter.fetch_add(SHARED_COUNTER, Ordering::Release);
    }

    /// Remove `N` from SHARED_COUNTER
    /// Add `N` to `LOCAL_COUNTER`
    ///
    /// SHARED_COUNTER -> LOCAL_COUNTER
    pub fn move_shared_to_local(&self, n: Counter) {
        let shared = n.shared();
        self.counter.fetch_add(
            (shared << SHARED_COUNTER_BIT_SIZE) - shared,
            Ordering::Release,
        );
    }

    #[inline]
    pub fn load(&self) -> Counter {
        Counter(self.counter.load(Ordering::Acquire))
    }
}

impl fmt::Debug for Counter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Counter {{ local: {}, shared: {} }}",
            self.local(),
            self.shared()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_counter() {
        let counter = TaskCounter::new();
        counter.increase_local();
        counter.decrease_local();
        assert_eq!(counter.load().total(), 0);
    }

    #[test]
    fn test_shared_counter() {
        let counter = TaskCounter::new();
        assert_eq!(counter.increase_local().shared(), 0);
        counter.increase_shared();
        assert_eq!(counter.increase_local().shared(), 1);
        assert_eq!(counter.decrease_local().shared(), 1);
        assert_eq!(counter.load().total(), 2);
    }

    #[test]
    fn test_move_counter() {
        let counter = TaskCounter::new();
        counter.increase_local();
        counter.move_shared_to_local(counter.load());
        assert_eq!(counter.load().total(), 1);

        counter.increase_shared();
        counter.move_shared_to_local(counter.increase_local());

        assert_eq!(counter.load().shared(), 0);
        assert_eq!(counter.load().local(), 3);
    }
}
