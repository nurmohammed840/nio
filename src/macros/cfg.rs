#![allow(unused_macros)]

macro_rules! feature {
    (
        #![$meta:meta]
        $($item:item)*
    ) => {
        $(
            #[cfg($meta)]
            #[cfg_attr(docsrs, doc(cfg($meta)))]
            $item
        )*
    }
}

/// Enables Windows-specific code.
/// Use this macro instead of `cfg(windows)` to generate docs properly.
macro_rules! cfg_windows {
    ($($item:item)*) => {
        $(
            #[cfg(any(all(doc, docsrs), windows))]
            #[cfg_attr(docsrs, doc(cfg(windows)))]
            $item
        )*
    }
}

/// Enables Unix-specific code.
/// Use this macro instead of `cfg(unix)` to generate docs properly.
macro_rules! cfg_unix {
    ($($item:item)*) => {
        $(
            #[cfg(any(all(doc, docsrs), unix))]
            #[cfg_attr(docsrs, doc(cfg(unix)))]
            $item
        )*
    }
}

macro_rules! cfg_aio {
    ($($item:item)*) => {
        $(
            #[cfg(all(any(docsrs, target_os = "freebsd"), feature = "net"))]
            #[cfg_attr(docsrs,
                doc(cfg(all(target_os = "freebsd", feature = "net")))
            )]
            $item
        )*
    }
}

macro_rules! cfg_fs {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "fs")]
            #[cfg_attr(docsrs, doc(cfg(feature = "fs")))]
            $item
        )*
    }
}

macro_rules! cfg_macros {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "macros")]
            #[cfg_attr(docsrs, doc(cfg(feature = "macros")))]
            $item
        )*
    }
}

macro_rules! cfg_net {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "net")]
            #[cfg_attr(docsrs, doc(cfg(feature = "net")))]
            $item
        )*
    }
}
