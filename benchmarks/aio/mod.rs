#![allow(warnings)]

#[cfg(feature = "tokio")]
pub use tokio::spawn;

#[cfg(not(feature = "tokio"))]
pub use nio::spawn;

pub use nio_future::yield_now;

#[cfg(not(feature = "tokio"))]
pub fn rt() -> nio::Runtime {
    nio::RuntimeBuilder::new().rt().unwrap()
}