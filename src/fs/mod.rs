// #![cfg(not(loom))]

//! Asynchronous file utilities.
//!
//! This module contains utility methods for working with the file system
//! asynchronously. This includes reading/writing to files, and working with
//! directories.
//!
//! Be aware that most operating systems do not provide asynchronous file system
//! APIs. Because of that, Nio will use ordinary blocking file operations
//! behind the scenes. This is done using the [`spawn_blocking`] threadpool to
//! run them in the background.
//!
//! The `nio::fs` module should only be used for ordinary files. Trying to use
//! it with e.g., a named pipe on Linux can result in surprising behavior,
//! such as hangs during runtime shutdown. For special files, you should use a
//! dedicated type such as [`nio::net::unix::pipe`] or [`AsyncFd`] instead.
//!
//! Currently, Nio will always use [`spawn_blocking`] on all platforms, but it
//! may be changed to use asynchronous file system APIs such as io_uring in the
//! future.
//!
//! # Usage
//!
//! The easiest way to use this module is to use the utility functions that
//! operate on entire files:
//!
//!  * [`nio::fs::read`](fn@crate::fs::read)
//!  * [`nio::fs::read_to_string`](fn@crate::fs::read_to_string)
//!  * [`nio::fs::write`](fn@crate::fs::write)
//!
//! The two `read` functions reads the entire file and returns its contents.
//! The `write` function takes the contents of the file and writes those
//! contents to the file. It overwrites the existing file, if any.
//!
//! For example, to read the file:
//!
//! ```
//! # async fn dox() -> std::io::Result<()> {
//! let contents = nio::fs::read_to_string("my_file.txt").await?;
//!
//! println!("File has {} lines.", contents.lines().count());
//! # Ok(())
//! # }
//! ```
//!
//! To overwrite the file:
//!
//! ```
//! # async fn dox() -> std::io::Result<()> {
//! let contents = "First line.\nSecond line.\nThird line.\n";
//!
//! nio::fs::write("my_file.txt", contents.as_bytes()).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Using `File`
//!
//! The main type for interacting with files is [`File`]. It can be used to read
//! from and write to a given file. This is done using the [`AsyncRead`] and
//! [`AsyncWrite`] traits. This type is generally used when you want to do
//! something more complex than just reading or writing the entire contents in
//! one go.
//!
//! **Note:** It is important to use [`flush`] when writing to a Nio
//! [`File`]. This is because calls to `write` will return before the write has
//! finished, and [`flush`] will wait for the write to finish. (The write will
//! happen even if you don't flush; it will just happen later.) This is
//! different from [`std::fs::File`], and is due to the fact that `File` uses
//! `spawn_blocking` behind the scenes.
//!
//! For example, to count the number of lines in a file without loading the
//! entire file into memory:
//!
//! ## Tuning your file IO
//!
//! Nio's file uses [`spawn_blocking`] behind the scenes, and this has serious
//! performance consequences. To get good performance with file IO on Nio, it
//! is recommended to batch your operations into as few `spawn_blocking` calls
//! as possible.
//!
//! One example of this difference can be seen by comparing the two reading
//! examples above. The first example uses [`nio::fs::read`], which reads the
//! entire file in a single `spawn_blocking` call, and then returns it. The
//! second example will read the file in chunks using many `spawn_blocking`
//! calls. This means that the second example will most likely be more expensive
//! for large files. (Of course, using chunks may be necessary for very large
//! files that don't fit in memory.)
//!
//! The following examples will show some strategies for this:
//!
//! When creating a file, write the data to a `String` or `Vec<u8>` and then
//! write the entire file in a single `spawn_blocking` call with
//! `nio::fs::write`.
//!
//! ```no_run
//! # async fn dox() -> std::io::Result<()> {
//! let mut contents = String::new();
//!
//! contents.push_str("First line.\n");
//! contents.push_str("Second line.\n");
//! contents.push_str("Third line.\n");
//!
//! nio::fs::write("my_file.txt", contents.as_bytes()).await?;
//! # Ok(())
//! # }
//! ```
//!
//! Manually use [`std::fs`] inside [`spawn_blocking`].
//!
//! ```no_run
//! use std::fs::File;
//! use std::io::{self, Write};
//! use nio::task::spawn_blocking;
//!
//! # async fn dox() -> std::io::Result<()> {
//! spawn_blocking(move || {
//!     let mut file = File::create("my_file.txt")?;
//!
//!     file.write_all(b"First line.\n")?;
//!     file.write_all(b"Second line.\n")?;
//!     file.write_all(b"Third line.\n")?;
//!
//!     // Unlike Nio's file, the std::fs file does
//!     // not need flush.
//!
//!     io::Result::Ok(())
//! }).await.unwrap()?;
//! # Ok(())
//! # }
//! ```
//!
//! It's also good to be aware of [`File::set_max_buf_size`], which controls the
//! maximum amount of bytes that Nio's [`File`] will read or write in a single
//! [`spawn_blocking`] call. The default is two megabytes, but this is subject
//! to change.
//!
//! [`spawn_blocking`]: fn@crate::task::spawn_blocking
//! [`AsyncRead`]: trait@crate::io::AsyncRead
//! [`AsyncWrite`]: trait@crate::io::AsyncWrite
//! [`BufReader`]: struct@crate::io::BufReader
//! [`BufWriter`]: struct@crate::io::BufWriter
//! [`nio::net::unix::pipe`]: crate::net::unix::pipe
//! [`AsyncFd`]: crate::io::unix::AsyncFd
//! [`nio::fs::read`]: fn@crate::fs::read

mod canonicalize;
pub use self::canonicalize::canonicalize;

mod create_dir;
pub use self::create_dir::create_dir;

mod create_dir_all;
pub use self::create_dir_all::create_dir_all;

mod dir_builder;
pub use self::dir_builder::DirBuilder;

// mod file;
// pub use self::file::File;

mod hard_link;
pub use self::hard_link::hard_link;

mod metadata;
pub use self::metadata::metadata;

// mod open_options;
// pub use self::open_options::OpenOptions;

mod read;
pub use self::read::read;

mod read_dir;
pub use self::read_dir::{read_dir, DirEntry, ReadDir};

mod read_link;
pub use self::read_link::read_link;

mod read_to_string;
pub use self::read_to_string::read_to_string;

mod remove_dir;
pub use self::remove_dir::remove_dir;

mod remove_dir_all;
pub use self::remove_dir_all::remove_dir_all;

mod remove_file;
pub use self::remove_file::remove_file;

mod rename;
pub use self::rename::rename;

mod set_permissions;
pub use self::set_permissions::set_permissions;

mod symlink_metadata;
pub use self::symlink_metadata::symlink_metadata;

mod write;
pub use self::write::write;

mod copy;
pub use self::copy::copy;

mod try_exists;
pub use self::try_exists::try_exists;

// #[cfg(test)]
// mod mocks;

feature! {
    #![unix]

    mod symlink;
    pub use self::symlink::symlink;
}

cfg_windows! {
    mod symlink_dir;
    pub use self::symlink_dir::symlink_dir;

    mod symlink_file;
    pub use self::symlink_file::symlink_file;
}

use std::io;

// #[cfg(not(test))]
// use crate::blocking::spawn_blocking;
// #[cfg(test)]
// use mocks::spawn_blocking;

pub(crate) async fn asyncify<F, T>(f: F) -> io::Result<T>
where
    F: FnOnce() -> io::Result<T> + Send + 'static,
    T: Send + 'static,
{
    match crate::task::spawn_blocking(f).await {
        Ok(res) => res,
        Err(_) => Err(io::Error::other("background task failed")),
    }
}
