//! Time error types.
use std::fmt;

/// Errors returned by `Timeout`.
///
/// This error is returned when a timeout expires before the function was able
/// to finish.
#[derive(Debug, PartialEq, Eq)]
pub struct TimeoutError(());

impl TimeoutError {
    pub(crate) fn new() -> Self {
        TimeoutError(())
    }
}

impl fmt::Display for TimeoutError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        "deadline has elapsed".fmt(fmt)
    }
}

impl std::error::Error for TimeoutError {}
impl From<TimeoutError> for std::io::Error {
    fn from(_: TimeoutError) -> std::io::Error {
        std::io::ErrorKind::TimedOut.into()
    }
}
