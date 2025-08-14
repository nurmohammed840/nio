#![allow(dead_code)]
use std::any::Any;
use std::fmt;
use std::io;

pub struct JoinError {
    repr: Repr,
}

enum Repr {
    Cancelled,
    Panic(Box<dyn Any + Send + 'static>),
}

unsafe impl Sync for Repr {}

impl JoinError {
    pub(super) fn cancelled() -> JoinError {
        JoinError {
            repr: Repr::Cancelled,
        }
    }

    pub(super) fn panic(err: Box<dyn Any + Send + 'static>) -> JoinError {
        JoinError {
            repr: Repr::Panic(err),
        }
    }

    pub(super) fn from(panic_result: Result<(), Box<dyn Any + Send>>) -> JoinError {
        match panic_result {
            Ok(()) => JoinError::cancelled(),
            Err(err) => JoinError::panic(err),
        }
    }

    /// Returns true if the error was caused by the task being cancelled.
    ///
    /// See [the module level docs] for more information on cancellation.
    ///
    /// [the module level docs]: crate::task#cancellation
    pub fn is_cancelled(&self) -> bool {
        matches!(&self.repr, Repr::Cancelled)
    }

    /// Returns true if the error was caused by the task panicking.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::panic;
    ///
    /// #[nio::main]
    /// async fn main() {
    ///     let err = nio::spawn(async {
    ///         panic!("boom");
    ///     }).await.unwrap_err();
    ///
    ///     assert!(err.is_panic());
    /// }
    /// ```
    pub fn is_panic(&self) -> bool {
        matches!(&self.repr, Repr::Panic(_))
    }

    /// Consumes the join error, returning the object with which the task panicked.
    ///
    /// # Panics
    ///
    /// `into_panic()` panics if the `Error` does not represent the underlying
    /// task terminating with a panic. Use `is_panic` to check the error reason
    /// or `try_into_panic` for a variant that does not panic.
    ///
    /// # Examples
    ///
    /// ```should_panic
    /// use std::panic;
    ///
    /// #[nio::main]
    /// async fn main() {
    ///     let err = nio::spawn(async {
    ///         panic!("boom");
    ///     }).await.unwrap_err();
    ///
    ///     if err.is_panic() {
    ///         // Resume the panic on the main task
    ///         panic::resume_unwind(err.into_panic());
    ///     }
    /// }
    /// ```
    #[track_caller]
    pub fn into_panic(self) -> Box<dyn Any + Send + 'static> {
        self.try_into_panic()
            .expect("`JoinError` reason is not a panic.")
    }

    /// Consumes the join error, returning the object with which the task
    /// panicked if the task terminated due to a panic. Otherwise, `self` is
    /// returned.
    ///
    /// # Examples
    ///
    /// ```should_panic
    /// use std::panic;
    ///
    /// #[nio::main]
    /// async fn main() {
    ///     let err = nio::spawn(async {
    ///         panic!("boom");
    ///     }).await.unwrap_err();
    ///
    ///     if let Ok(reason) = err.try_into_panic() {
    ///         // Resume the panic on the main task
    ///         panic::resume_unwind(reason);
    ///     }
    /// }
    /// ```
    pub fn try_into_panic(self) -> Result<Box<dyn Any + Send + 'static>, JoinError> {
        match self.repr {
            Repr::Panic(p) => Ok(p),
            _ => Err(self),
        }
    }
}

impl std::error::Error for JoinError {}

impl fmt::Display for JoinError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.repr {
            Repr::Cancelled => write!(fmt, "task was cancelled"),
            Repr::Panic(p) => match panic_payload_as_str(p) {
                Some(panic_str) => {
                    write!(fmt, "task panicked with message {:?}", panic_str)
                }
                None => {
                    write!(fmt, "task panicked")
                }
            },
        }
    }
}

impl fmt::Debug for JoinError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.repr {
            Repr::Cancelled => write!(fmt, "JoinError::Cancelled"),
            Repr::Panic(p) => match panic_payload_as_str(p) {
                Some(panic_str) => {
                    write!(fmt, "JoinError::Panic({:?}, ...)", panic_str)
                }
                None => write!(fmt, "JoinError::Panic(...)"),
            },
        }
    }
}

impl From<JoinError> for io::Error {
    fn from(src: JoinError) -> io::Error {
        io::Error::other(match src.repr {
            Repr::Cancelled => "task was cancelled",
            Repr::Panic(_) => "task panicked",
        })
    }
}

fn panic_payload_as_str(payload: &Box<dyn Any + Send>) -> Option<&str> {
    // Panic payloads are almost always `String` (if invoked with formatting arguments)
    // or `&'static str` (if invoked with a string literal).
    //
    // Non-string panic payloads have niche use-cases,
    // so we don't really need to worry about those.
    if let Some(s) = payload.downcast_ref::<String>() {
        return Some(s);
    }

    if let Some(s) = payload.downcast_ref::<&'static str>() {
        return Some(s);
    }

    None
}
