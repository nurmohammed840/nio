use std::{fmt, num::NonZeroUsize, sync::Arc};

use super::raw::RawTask;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct Id(pub(crate) NonZeroUsize);

impl Id {
    pub(super) fn new(task: &RawTask) -> Self {
        Self(unsafe { NonZeroUsize::new_unchecked(Arc::as_ptr(task).cast::<()>() as usize) })
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
