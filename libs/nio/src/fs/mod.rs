#![allow(unused_macros)]
macro_rules! feature {
    (
        #![$meta:meta]
        $($item:item)*
    ) => {
        $(
            #[cfg($meta)]
            $item
        )*
    }
}

/// Enables Windows-specific code.
/// Use this macro instead of `cfg(windows)` to generate docs properly.
macro_rules! cfg_windows {
    ($($item:item)*) => {
        $(
            #[cfg(windows)]
            $item
        )*
    }
}

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
pub use self::read_dir::{DirEntry, ReadDir, read_dir};

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
    match crate::spawn_blocking(f).await {
        Ok(res) => res,
        Err(_) => Err(io::Error::other("background task failed")),
    }
}
