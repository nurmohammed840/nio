use std::fmt;

use super::{COMPLETE, RawTask, id::Id};

#[derive(Clone)]
pub struct AbortHandle {
    raw: RawTask,
}

unsafe impl Send for AbortHandle {}
unsafe impl Sync for AbortHandle {}

impl AbortHandle {
    pub(super) fn new(raw: RawTask) -> Self {
        Self { raw }
    }

    pub fn abort(self) {
        unsafe {
            self.raw.abort_task();
        }
    }

    pub fn is_finished(&self) -> bool {
        self.raw.header().state.load().has(COMPLETE)
    }

    pub fn id(&self) -> Id {
        Id::new(&self.raw)
    }
}

impl std::panic::UnwindSafe for AbortHandle {}
impl std::panic::RefUnwindSafe for AbortHandle {}

impl fmt::Debug for AbortHandle {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("AbortHandle")
            .field("id", &self.id())
            .finish()
    }
}
