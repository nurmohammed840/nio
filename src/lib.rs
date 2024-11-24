#![doc = include_str!("../README.md")]
#![allow(clippy::manual_unwrap_or_default, clippy::new_without_default)]

// Includes re-exports used by macros.
//
// This module is not intended to be part of the public API. In general, any
// `doc(hidden)` code is not part of Nio's public and stable API.
#[macro_use]
mod macros;

pub mod fs;
pub mod net;
pub mod task;

mod blocking;
mod future;
mod scheduler;

pub mod io;
pub mod runtime;
pub mod time;

/// Boundary value to prevent stack overflow caused by a large-sized
/// Future being placed in the stack.
pub(crate) const BOX_FUTURE_THRESHOLD: usize = if cfg!(debug_assertions) { 2048 } else { 16384 };

pub use task::spawn;

pub use nio_macros::*;
