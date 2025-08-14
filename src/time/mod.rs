mod error;
mod timeout;

pub(crate) mod timer;

pub use error::TimeoutError;
pub use std::time::Duration;
pub use timeout::{timeout, Timeout};
pub use timer::{sleep, Sleep};
