mod error;
mod timeout;

pub(crate) mod timer;

pub use error::TimeoutError;
pub use timeout::{timeout, Timeout};
pub use timer::{sleep, Sleep};
pub use std::time::Duration;