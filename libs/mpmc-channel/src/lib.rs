use std::{
    fmt,
    ops::{Deref, DerefMut},
    sync::{Condvar, Mutex, MutexGuard, TryLockError},
    time::Duration,
};

#[derive(Debug, Default)]
struct Inner<T> {
    waiter: u32,
    data: T,
}

#[derive(Debug, Default)]
pub struct MPMC<T> {
    cvar: Condvar,
    inner: Mutex<Inner<T>>,
}

pub struct Consume<'a, T> {
    cvar: &'a Condvar,
    guard: MutexGuard<'a, Inner<T>>,
}

pub struct Producer<'a, T> {
    cvar: &'a Condvar,
    guard: MutexGuard<'a, Inner<T>>,
}

#[derive(Debug, Clone, Copy)]
pub struct WouldBlock;

#[derive(Debug, Clone, Copy)]
pub struct WaitTimeOut;

impl<T> MPMC<T> {
    #[inline]
    pub const fn new(data: T) -> Self {
        Self {
            cvar: Condvar::new(),
            inner: Mutex::new(Inner { data, waiter: 0 }),
        }
    }

    #[inline]
    pub fn produce(&self) -> Producer<T> {
        let guard = self.inner.lock().unwrap();
        Producer {
            cvar: &self.cvar,
            guard,
        }
    }

    #[inline]
    pub fn consume(&self) -> Consume<T> {
        let guard = self.inner.lock().unwrap();
        Consume {
            cvar: &self.cvar,
            guard,
        }
    }

    pub fn try_produce(&self) -> Result<Producer<T>, WouldBlock> {
        let guard = match self.inner.try_lock() {
            Ok(val) => val,
            Err(TryLockError::WouldBlock) => return Err(WouldBlock),
            Err(err) => panic!("{err}"),
        };
        Ok(Producer {
            cvar: &self.cvar,
            guard,
        })
    }

    pub fn try_consume(&self) -> Result<Consume<T>, WouldBlock> {
        let guard = match self.inner.try_lock() {
            Ok(val) => val,
            Err(TryLockError::WouldBlock) => return Err(WouldBlock),
            Err(err) => panic!("{err}"),
        };
        Ok(Consume {
            cvar: &self.cvar,
            guard,
        })
    }
}

impl<T> Consume<'_, T> {
    pub fn wait(mut self) -> Self {
        self.guard.waiter += 1;
        self.guard = self.cvar.wait(self.guard).unwrap();
        self.guard.waiter -= 1;
        self
    }

    pub fn wait_timeout(mut self, dur: Duration) -> Result<Self, WaitTimeOut> {
        self.guard.waiter += 1;
        let result = self.cvar.wait_timeout(self.guard, dur).unwrap();
        self.guard = result.0;
        self.guard.waiter -= 1;
        if result.1.timed_out() {
            Err(WaitTimeOut)
        } else {
            Ok(self)
        }
    }
}

impl<T> Producer<'_, T> {
    pub fn notify_one(self) {
        if self.guard.waiter != 0 {
            drop(self.guard);
            self.cvar.notify_one();
        }
    }

    pub fn notify_all(self) {
        if self.guard.waiter != 0 {
            drop(self.guard);
            self.cvar.notify_all();
        }
    }
}

impl<T> Deref for Consume<'_, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.guard.data
    }
}

impl<T> DerefMut for Consume<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard.data
    }
}

impl<T> Deref for Producer<'_, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.guard.data
    }
}

impl<T> DerefMut for Producer<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard.data
    }
}

impl std::error::Error for WaitTimeOut {}
impl fmt::Display for WaitTimeOut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
