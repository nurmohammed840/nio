use std::{
    fmt,
    sync::atomic::{AtomicU64, Ordering},
};

/// ## Bit layout
///
/// ```md
/// 63                           33     31                    0
/// ┌─────────────────────────────┬─────┬─────────────────────┐
/// │        LOCAL_COUNTER        │  N  │    SHARED_COUNTER   │
/// └─────────────────────────────┴─────┴─────────────────────┘
///
/// SHARED_COUNTER → bits 0..=31
/// NOTIFIED_FLAG  → bit  32
/// LOCAL_COUNTER  → bits 33..=63
/// ```
const LOCAL_COUNTER_BIT_SIZE: u8 = 32;

const SHARED_COUNTER_BIT_SIZE: u8 = LOCAL_COUNTER_BIT_SIZE - 1;
const SHARED_COUNTER_MASK: u64 = (1 << SHARED_COUNTER_BIT_SIZE) - 1;
const SHARED_COUNTER_ONE: u64 = 1;

/// One bit is for `NOTIFIED_FLAG`
const NOTIFIED_FLAG: u64 = 1 << SHARED_COUNTER_BIT_SIZE;

const LOCAL_COUNTER_ONE: u64 = 1 << LOCAL_COUNTER_BIT_SIZE;

pub struct TaskQueue {
    counter: AtomicU64,
}

#[derive(Clone, Copy)]
#[must_use]
pub struct Counter(u64);

impl Counter {
    #[inline]
    pub fn local(self) -> u64 {
        self.0 >> LOCAL_COUNTER_BIT_SIZE
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

    #[inline]
    pub(crate) fn is_notified(self) -> bool {
        (self.0 & NOTIFIED_FLAG) == NOTIFIED_FLAG
    }
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            counter: AtomicU64::new(0),
        }
    }

    /// clear `NOTIFIED_FLAG`
    pub fn accept_notify_once_if_shared_queue_is_empty(&self) -> Counter {
        let result = self
            .counter
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |curr| {
                let curr = Counter(curr);
                // shared queue not empty → do nothing
                if curr.shared() != 0 {
                    return None;
                }
                // `NOTIFIED_FLAG` is not set
                if !curr.is_notified() {
                    return None;
                }
                // clear `NOTIFIED_FLAG`
                Some(curr.0 & !NOTIFIED_FLAG)
            });

        match result {
            Ok(state) | Err(state) => Counter(state),
        }
    }

    pub fn increase_local(&self) -> Counter {
        Counter(self.counter.fetch_add(LOCAL_COUNTER_ONE, Ordering::AcqRel))
    }

    pub fn decrease_local(&self) -> Counter {
        let old = Counter(self.counter.fetch_sub(LOCAL_COUNTER_ONE, Ordering::AcqRel));
        debug_assert!(old.local() > 0);
        old
    }

    pub fn clear_notified_flag(&self) {
        self.counter.fetch_and(!NOTIFIED_FLAG, Ordering::Release);
    }

    pub fn increase_shared_and_mark_as_notified(&self) -> Counter {
        let state = self
            .counter
            .fetch_update(Ordering::AcqRel, Ordering::Relaxed, |curr| {
                Some((curr | NOTIFIED_FLAG) + SHARED_COUNTER_ONE)
            })
            .unwrap();

        Counter(state)
    }

    /// Remove `N` from SHARED_COUNTER
    /// Add `N` to `LOCAL_COUNTER`
    ///
    /// SHARED_COUNTER -> LOCAL_COUNTER
    pub fn move_shared_to_local(&self, n: Counter) {
        let shared = n.shared();
        self.counter.fetch_add(
            (shared << LOCAL_COUNTER_BIT_SIZE) - shared,
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
            "Counter {{ local: {}, shared: {}, notified: {} }}",
            self.local(),
            self.shared(),
            self.is_notified()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl TaskQueue {
        /// Only this function is allowed to call from other thread.
        pub fn increase_shared(&self) -> Counter {
            self.increase_shared_and_mark_as_notified()
        }
    }

    #[test]
    fn test_local_counter() {
        let q = TaskQueue::new();
        assert_eq!(q.increase_local().total(), 0);
        assert_eq!(q.decrease_local().local(), 1);
        assert_eq!(q.load().total(), 0);
    }

    #[test]
    fn test_shared_counter() {
        let q = TaskQueue::new();
        assert_eq!(q.increase_local().shared(), 0);
        assert_eq!(q.increase_shared().local(), 1);
        assert_eq!(q.increase_local().total(), 2);
        assert_eq!(q.decrease_local().shared(), 1);
        assert_eq!(q.load().total(), 2);
    }

    #[test]
    fn test_move_counter() {
        let q = TaskQueue::new();
        assert_eq!(q.increase_local().total(), 0);
        q.move_shared_to_local(q.load());
        assert_eq!(q.load().total(), 1);

        assert_eq!(q.increase_shared().local(), 1);
        let old = q.increase_local();
        q.move_shared_to_local(old);

        assert_eq!(q.load().shared(), 0);
        assert_eq!(q.load().local(), 3);
    }

    #[test]
    fn test_notification_flag() {
        let q = TaskQueue::new();

        assert_eq!(q.load().is_notified(), false);
        assert!(!q.increase_shared_and_mark_as_notified().is_notified());

        let old = q.increase_local();
        assert_eq!(old.local(), 0);
        assert_eq!(old.shared(), 1);
        assert_eq!(old.is_notified(), true); // flag unaffected

        // Attempt to clear NOTIFIED_FLAG while shared is not empty
        let old = q.accept_notify_once_if_shared_queue_is_empty();
        assert_eq!(old.is_notified(), true);
        assert_eq!(q.load().is_notified(), true); // flag unaffected

        // clear shared counter
        q.move_shared_to_local(old);

        // Now shared is empty, clearing `NOTIFIED_FLAG` should succeed.
        let old = q.accept_notify_once_if_shared_queue_is_empty();
        assert_eq!(old.local(), 2);
        assert_eq!(old.shared(), 0);
        assert_eq!(old.is_notified(), true);

        // Mark as notified again
        let old = q.increase_shared_and_mark_as_notified();
        assert_eq!(old.is_notified(), false);

        let curr = q.load();
        assert_eq!(curr.local(), 2);
        assert_eq!(curr.shared(), 1);
        assert_eq!(curr.is_notified(), true);

        q.clear_notified_flag();
        assert_eq!(q.load().is_notified(), false);

        let old = q.increase_shared_and_mark_as_notified();
        assert_eq!(old.shared(), 1);
        assert_eq!(old.is_notified(), false);

        // Increase local
        let old = q.increase_local();
        assert_eq!(old.local(), 2);
        assert_eq!(old.is_notified(), true);

        let curr = q.load();
        assert_eq!(curr.local(), 3);
        assert_eq!(curr.shared(), 2);
        assert_eq!(curr.is_notified(), true);
    }
}
