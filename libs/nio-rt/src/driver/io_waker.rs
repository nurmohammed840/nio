use std::cell::Cell;

use crate::local_waker::LocalWaker;

pub struct Readiness(u8);

impl Readiness {
    const READABLE: u8 = 0b_01;
    const READ_CLOSED: u8 = 0b_10;
    const READ_MASK: u8 = Readiness::READABLE | Readiness::READ_CLOSED;

    const WRITABLE: u8 = 0b_01_00;
    const WRITE_CLOSED: u8 = 0b_10_00;
    const WRITE_MASK: u8 = Readiness::WRITABLE | Readiness::WRITE_CLOSED;

    #[inline]
    pub fn is_readable(&self) -> bool {
        self.0 & Readiness::READ_MASK != 0
    }

    #[inline]
    pub fn is_writable(&self) -> bool {
        self.0 & Readiness::WRITE_MASK != 0
    }
}

#[derive(Default, Debug)]
pub struct IoWaker {
    readiness: Cell<u8>,
    pub reader: LocalWaker,
    pub writer: LocalWaker,
}

impl IoWaker {
    #[inline]
    pub fn new() -> Box<IoWaker> {
        Box::new(Self::default())
    }

    #[inline]
    pub fn addr(self: &Box<Self>) -> usize {
        let ptr = &raw const **self;
        ptr.expose_provenance()
    }

    #[inline]
    pub fn from(addr: usize) -> *const IoWaker {
        std::ptr::with_exposed_provenance(addr)
    }

    #[inline]
    pub fn readiness(&self) -> Readiness {
        Readiness(self.readiness.get())
    }

    #[inline]
    pub fn clear_read(&self, readiness: Readiness) {
        self.readiness.set(readiness.0 & !Readiness::READABLE);
    }

    #[inline]
    pub fn clear_write(&self, readiness: Readiness) {
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
            self.reader.wake();
        }
        if readiness & Readiness::WRITE_MASK != 0 {
            self.writer.wake();
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

    pub fn drop_waker(&self) {
        self.reader.take();
        self.writer.take();
    }
}

impl Drop for IoWaker {
    fn drop(&mut self) {
        self.reader.wake();
        self.writer.wake();
    }
}
