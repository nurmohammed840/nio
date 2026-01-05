use std::{cell::Cell, rc::Rc};

use crate::local_waker::LocalWaker;

struct Readiness(u8);

impl Readiness {
    const READABLE: u8 = 0b_01;
    const READ_CLOSED: u8 = 0b_10;
    const READ_MASK: u8 = Readiness::READABLE | Readiness::READ_CLOSED;

    const WRITABLE: u8 = 0b_01_00;
    const WRITE_CLOSED: u8 = 0b_10_00;
    const WRITE_MASK: u8 = Readiness::WRITABLE | Readiness::WRITE_CLOSED;

    #[inline]
    fn is_readable(&self) -> bool {
        self.0 & Readiness::READ_MASK != 0
    }

    #[inline]
    fn is_writable(&self) -> bool {
        self.0 & Readiness::WRITE_MASK != 0
    }
}

#[derive(Default)]
pub struct IoWaker {
    pub readiness: Cell<u8>,
    pub read_waker: LocalWaker,
    pub write_waker: LocalWaker,
}

impl IoWaker {
    #[inline]
    pub fn new() -> Rc<IoWaker> {
        Rc::new(Self::default())
    }

    #[inline]
    pub fn addr(self: &Rc<Self>) -> usize {
        Rc::as_ptr(self).expose_provenance()
    }

    #[inline]
    pub fn from(addr: usize) -> *const IoWaker {
        std::ptr::with_exposed_provenance(addr)
    }

    #[inline]
    fn readiness(&self) -> Readiness {
        Readiness(self.readiness.get())
    }

    #[inline]
    fn clear_read(&self, readiness: Readiness) {
        self.readiness.set(readiness.0 & !Readiness::READABLE);
    }

    #[inline]
    fn clear_write(&self, readiness: Readiness) {
        self.readiness.set(readiness.0 & !Readiness::WRITABLE);
    }

    pub fn notify(&self, ev: &mio::event::Event) {
        let mut readiness: u8 = self.readiness.get();

        #[cfg(all(target_os = "freebsd"))]
        {
            if ev.is_aio() {
                readiness |= Readiness::READABLE;
            }
            if ev.is_lio() {
                readiness |= Readiness::READABLE;
            }
        }

        if ev.is_readable() {
            readiness |= Readiness::READABLE;
        }
        if ev.is_read_closed() {
            readiness |= Readiness::READ_CLOSED;
        }

        if ev.is_writable() {
            readiness |= Readiness::WRITABLE;
        }
        if ev.is_write_closed() {
            readiness |= Readiness::WRITE_CLOSED;
        }

        self.readiness.set(readiness);

        if readiness & Readiness::READ_MASK != 0 {
            self.read_waker.wake();
        }
        if readiness & Readiness::WRITE_MASK != 0 {
            self.write_waker.wake();
        }

        #[cfg(debug_assertions)]
        if readiness == 0 {
            if ev.is_error() {
                eprintln!("error without readiness: {ev:#?}");
            } else {
                eprintln!("without readiness: {ev:#?}");
            }
        }
    }

    fn drop_waker(&self) {
        self.read_waker.take();
        self.write_waker.take();
    }
}

impl Drop for IoWaker {
    fn drop(&mut self) {
        self.read_waker.wake();
        self.write_waker.wake();
    }
}
