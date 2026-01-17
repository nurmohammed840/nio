#![allow(unused)]

#[cfg(not(feature = "tokio"))]
pub mod nio;

#[cfg(feature = "tokio")]
pub mod tokio;

#[cfg(not(feature = "tokio"))]
pub use nio::*;

#[cfg(feature = "tokio")]
pub use tokio::*;
