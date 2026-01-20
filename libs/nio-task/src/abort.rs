use std::fmt;

use super::{COMPLETE, RawTask, id::TaskId};

#[derive(Clone)]
pub struct AbortHandle {
    pub(super) raw: RawTask,
}

unsafe impl Send for AbortHandle {}
unsafe impl Sync for AbortHandle {}

impl AbortHandle {
    pub fn abort(&self) {
        unsafe {
            self.raw.abort_task(self.raw.clone());
        }
    }

    pub fn is_finished(&self) -> bool {
        self.raw.header().state.load().has(COMPLETE)
    }

    pub fn id(&self) -> TaskId {
        TaskId::new(&self.raw)
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
