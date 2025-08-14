use futures::task::AtomicWaker;

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::{AcqRel, Acquire};
use std::sync::Arc;

// # This struct should be cache padded to avoid false sharing.
// see: https://docs.rs/crossbeam/latest/crossbeam/utils/struct.CachePadded.html
#[cfg_attr(
    any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "arm64ec",
        target_arch = "powerpc64",
    ),
    repr(align(128))
)]
#[cfg_attr(
    any(
        target_arch = "arm",
        target_arch = "mips",
        target_arch = "mips32r6",
        target_arch = "mips64",
        target_arch = "mips64r6",
        target_arch = "sparc",
        target_arch = "hexagon",
    ),
    repr(align(32))
)]
#[cfg_attr(target_arch = "m68k", repr(align(16)))]
#[cfg_attr(target_arch = "s390x", repr(align(256)))]
#[cfg_attr(
    not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "arm64ec",
        target_arch = "powerpc64",
        target_arch = "arm",
        target_arch = "mips",
        target_arch = "mips32r6",
        target_arch = "mips64",
        target_arch = "mips64r6",
        target_arch = "sparc",
        target_arch = "hexagon",
        target_arch = "m68k",
        target_arch = "s390x",
    )),
    repr(align(64))
)]
#[derive(Default)]
pub struct ScheduledIo {
    /// Packs the resource's readiness and I/O driver latest tick.
    pub read_event: Event,
    pub write_event: Event,
}

#[derive(Default)]
pub struct Event {
    readiness: AtomicUsize,
    pub waker: AtomicWaker,
}

impl Event {
    fn update_readiness_and_notify(&self, readiness: u16) {
        let _ = self.readiness.fetch_update(AcqRel, Acquire, |state| {
            let mut state = ReadinessVersion::from_usize(state);
            state.version = state.version.wrapping_add(1);
            state.readiness |= readiness;
            Some(state.into_usize())
        });
        self.waker.wake();
    }
}

mod readiness {
    pub const READABLE: u16 = 0b_01;
    pub const WRITABLE: u16 = 0b_01;

    pub const READ_CLOSED: u16 = 0b_10;
    pub const WRITE_CLOSED: u16 = 0b_10;
}

#[derive(Debug)]
pub struct Readiness(usize);

impl Readiness {
    pub fn is_empty(&self) -> bool {
        (self.0 & 0xFFFF) == 0
    }

    pub fn is_read_closed(&self) -> bool {
        self.0 & (readiness::READ_CLOSED as usize) == (readiness::READ_CLOSED as usize)
    }

    pub fn is_write_closed(&self) -> bool {
        self.0 & (readiness::WRITE_CLOSED as usize) == (readiness::WRITE_CLOSED as usize)
    }
}

// |  version  | readiness |
// |-----------+-----------|
// |  16 bits  +  16 bits  |
struct ReadinessVersion {
    version: u16,
    readiness: u16,
}

impl ReadinessVersion {
    pub fn from_usize(state: usize) -> Self {
        Self {
            version: ((state >> 16) & 0xFFFF) as u16,
            readiness: (state & 0xFFFF) as u16,
        }
    }

    fn into_usize(self) -> usize {
        let version = (self.version as usize) << 16;
        let readiness = self.readiness as usize;
        version | readiness
    }
}

impl ScheduledIo {
    #[inline]
    pub fn into_token(self: &Arc<Self>) -> usize {
        Arc::as_ptr(self).expose_provenance()
    }

    #[inline]
    pub fn from_token(token: usize) -> *const ScheduledIo {
        std::ptr::with_exposed_provenance(token)
    }

    pub fn read_readiness(&self) -> Readiness {
        Readiness(self.read_event.readiness.load(Acquire))
    }

    pub fn write_readiness(&self) -> Readiness {
        Readiness(self.write_event.readiness.load(Acquire))
    }

    pub fn notify_event(&self, ev: &mio::event::Event) {
        let mut notify_read: u16 = 0;
        let mut notify_write: u16 = 0;

        #[cfg(all(target_os = "freebsd", feature = "net"))]
        {
            if ev.is_aio() {
                notify_read |= readiness::READABLE;
            }
            if ev.is_lio() {
                notify_read |= readiness::READABLE;
            }
        }

        if ev.is_readable() {
            notify_read |= readiness::READABLE;
        }
        if ev.is_read_closed() {
            notify_read |= readiness::READ_CLOSED;
        }

        if ev.is_writable() {
            notify_write |= readiness::WRITABLE;
        }
        if ev.is_write_closed() {
            notify_write |= readiness::WRITE_CLOSED;
        }

        if notify_read != 0 {
            self.read_event.update_readiness_and_notify(notify_read);
        }
        if notify_write != 0 {
            self.write_event.update_readiness_and_notify(notify_write);
        }

        #[cfg(debug_assertions)]
        if ev.is_error() && notify_read == 0 && notify_write == 0 {
            eprintln!("error without readiness: {ev:#?}");
        }
    }

    pub fn clear_read_readiness(&self, old: Readiness) {
        let new = old.0 & !(readiness::READABLE as usize);
        let _ = self
            .read_event
            .readiness
            .compare_exchange(old.0, new, AcqRel, Acquire);
    }

    pub fn clear_write_readiness(&self, old: Readiness) {
        let new = old.0 & !(readiness::WRITABLE as usize);
        let _ = self
            .write_event
            .readiness
            .compare_exchange(old.0, new, AcqRel, Acquire);
    }

    pub fn drop_wakers(&self) {
        self.read_event.waker.take();
        self.write_event.waker.take();
    }
}

impl Drop for ScheduledIo {
    fn drop(&mut self) {
        self.read_event.waker.wake();
        self.write_event.waker.wake();
    }
}
