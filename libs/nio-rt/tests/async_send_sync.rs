#![warn(rust_2018_idioms)]
#![allow(clippy::type_complexity, clippy::diverging_sub_expression)]

use std::cell::Cell;
use std::net::SocketAddr;
use std::rc::Rc;
use std::time::{Duration, Instant};

// The names of these structs behaves better when sorted.
// Send: Yes, Sync: Yes
#[derive(Clone)]
struct YY {}

// Send: Yes, Sync: No
#[derive(Clone)]
struct YN {
    _value: Cell<u8>,
}

// Send: No, Sync: No
#[derive(Clone)]
struct NN {
    _value: Rc<u8>,
}

type BoxFutureSync<T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + Sync>>;
type BoxFutureSend<T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send>>;
type BoxFuture<T> = std::pin::Pin<Box<dyn std::future::Future<Output = T>>>;

fn require_send<T: Send>(_t: &T) {}
fn require_sync<T: Sync>(_t: &T) {}
fn require_unpin<T: Unpin>(_t: &T) {}

struct Invalid;

trait AmbiguousIfSend<A> {
    fn some_item(&self) {}
}
impl<T: ?Sized> AmbiguousIfSend<()> for T {}
impl<T: ?Sized + Send> AmbiguousIfSend<Invalid> for T {}

trait AmbiguousIfSync<A> {
    fn some_item(&self) {}
}
impl<T: ?Sized> AmbiguousIfSync<()> for T {}
impl<T: ?Sized + Sync> AmbiguousIfSync<Invalid> for T {}

trait AmbiguousIfUnpin<A> {
    fn some_item(&self) {}
}
impl<T: ?Sized> AmbiguousIfUnpin<()> for T {}
impl<T: ?Sized + Unpin> AmbiguousIfUnpin<Invalid> for T {}

macro_rules! into_todo {
    ($typ:ty) => {{
        let x: $typ = todo!();
        x
    }};
}

macro_rules! async_assert_fn_send {
    (Send & $(!)?Sync & $(!)?Unpin, $value:expr) => {
        require_send(&$value);
    };
    (!Send & $(!)?Sync & $(!)?Unpin, $value:expr) => {
        AmbiguousIfSend::some_item(&$value);
    };
}
macro_rules! async_assert_fn_sync {
    ($(!)?Send & Sync & $(!)?Unpin, $value:expr) => {
        require_sync(&$value);
    };
    ($(!)?Send & !Sync & $(!)?Unpin, $value:expr) => {
        AmbiguousIfSync::some_item(&$value);
    };
}
macro_rules! async_assert_fn_unpin {
    ($(!)?Send & $(!)?Sync & Unpin, $value:expr) => {
        require_unpin(&$value);
    };
    ($(!)?Send & $(!)?Sync & !Unpin, $value:expr) => {
        AmbiguousIfUnpin::some_item(&$value);
    };
}

macro_rules! async_assert_fn {
    ($($f:ident $(< $($generic:ty),* > )? )::+($($arg:ty),*): $($tok:tt)*) => {
        #[allow(unreachable_code)]
        #[allow(unused_variables)]
        const _: fn() = || {
            let f = $($f $(::<$($generic),*>)? )::+( $( into_todo!($arg) ),* );
            async_assert_fn_send!($($tok)*, f);
            async_assert_fn_sync!($($tok)*, f);
            async_assert_fn_unpin!($($tok)*, f);
        };
    };
}
macro_rules! assert_value {
    ($type:ty: $($tok:tt)*) => {
        #[allow(unreachable_code)]
        #[allow(unused_variables)]
        const _: fn() = || {
            let f: $type = todo!();
            async_assert_fn_send!($($tok)*, f);
            async_assert_fn_sync!($($tok)*, f);
            async_assert_fn_unpin!($($tok)*, f);
        };
    };
}

macro_rules! cfg_not_wasi {
    ($($item:item)*) => {
        $(
            #[cfg(not(target_os = "wasi"))]
            $item
        )*
    }
}

// Manually re-implementation of `async_assert_fn` for `poll_fn`. The macro
// doesn't work for this particular case because constructing the closure
// is too complicated.
const _: fn() = || {
    let _pinned = std::marker::PhantomPinned;
    let f = tokio::macros::support::poll_fn(move |_| {
        // Use `pinned` to take ownership of it.
        let _ = &_pinned;
        std::task::Poll::Pending::<()>
    });
    require_send(&f);
    require_sync(&f);
    AmbiguousIfUnpin::some_item(&f);
};

cfg_not_wasi! {
    mod fs {
        use super::*;
        assert_value!(nio_rt::fs::DirBuilder: Send & Sync & Unpin);
        assert_value!(nio_rt::fs::DirEntry: Send & Sync & Unpin);
        // assert_value!(nio_rt::fs::File: Send & Sync & Unpin);
        // assert_value!(nio_rt::fs::OpenOptions: Send & Sync & Unpin);
        assert_value!(nio_rt::fs::ReadDir: Send & Sync & Unpin);

        async_assert_fn!(nio_rt::fs::canonicalize(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio_rt::fs::copy(&str, &str): Send & Sync & !Unpin);
        async_assert_fn!(nio_rt::fs::create_dir(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio_rt::fs::create_dir_all(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio_rt::fs::hard_link(&str, &str): Send & Sync & !Unpin);
        async_assert_fn!(nio_rt::fs::metadata(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio_rt::fs::read(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio_rt::fs::read_dir(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio_rt::fs::read_link(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio_rt::fs::read_to_string(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio_rt::fs::remove_dir(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio_rt::fs::remove_dir_all(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio_rt::fs::remove_file(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio_rt::fs::rename(&str, &str): Send & Sync & !Unpin);
        async_assert_fn!(nio_rt::fs::set_permissions(&str, std::fs::Permissions): Send & Sync & !Unpin);
        async_assert_fn!(nio_rt::fs::symlink_metadata(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio_rt::fs::write(&str, Vec<u8>): Send & Sync & !Unpin);
        async_assert_fn!(nio_rt::fs::ReadDir::next_entry(_): Send & Sync & !Unpin);
        // async_assert_fn!(nio_rt::fs::OpenOptions::open(_, &str): Send & Sync & !Unpin);
        async_assert_fn!(nio_rt::fs::DirBuilder::create(_, &str): Send & Sync & !Unpin);
        async_assert_fn!(nio_rt::fs::DirEntry::metadata(_): Send & Sync & !Unpin);
        async_assert_fn!(nio_rt::fs::DirEntry::file_type(_): Send & Sync & !Unpin);
        // async_assert_fn!(nio_rt::fs::File::open(&str): Send & Sync & !Unpin);
        // async_assert_fn!(nio_rt::fs::File::create(&str): Send & Sync & !Unpin);
        // async_assert_fn!(nio_rt::fs::File::sync_all(_): Send & Sync & !Unpin);
        // async_assert_fn!(nio_rt::fs::File::sync_data(_): Send & Sync & !Unpin);
        // async_assert_fn!(nio_rt::fs::File::set_len(_, u64): Send & Sync & !Unpin);
        // async_assert_fn!(nio_rt::fs::File::metadata(_): Send & Sync & !Unpin);
        // async_assert_fn!(nio_rt::fs::File::try_clone(_): Send & Sync & !Unpin);
        // async_assert_fn!(nio_rt::fs::File::into_std(_): Send & Sync & !Unpin);
        // async_assert_fn!(
        //     nio_rt::fs::File::set_permissions(_, std::fs::Permissions): Send & Sync & !Unpin
        // );
    }
}

cfg_not_wasi! {
    // assert_value!(nio_rt::net::TcpSocket: !Send & !Sync & Unpin);
    async_assert_fn!(nio_rt::net::TcpListener::bind(SocketAddr): !Send & !Sync & Unpin);
    async_assert_fn!(nio_rt::net::TcpStream::connect(SocketAddr): !Send & !Sync & !Unpin);
}

assert_value!(nio_rt::net::TcpListener: !Send & !Sync & Unpin);
assert_value!(nio_rt::net::TcpStream: !Send & !Sync & Unpin);
assert_value!(nio_rt::net::TcpReader: !Send & !Sync & Unpin);
assert_value!(nio_rt::net::TcpWriter: !Send & !Sync & Unpin);
// assert_value!(nio_rt::net::tcp::ReadHalf<'_>: Send & Sync & Unpin);
// assert_value!(nio_rt::net::tcp::WriteHalf<'_>: Send & Sync & Unpin);
// assert_value!(nio_rt::net::tcp::ReuniteError: Send & Sync & Unpin);
async_assert_fn!(nio_rt::net::TcpListener::accept(_): !Send & !Sync & Unpin);
async_assert_fn!(nio_rt::net::TcpStream::peek(_, &mut [u8]): !Send & !Sync & Unpin);
// async_assert_fn!(nio_rt::net::TcpStream::readable(_): Send & Sync & !Unpin);
// async_assert_fn!(nio_rt::net::TcpStream::ready(_, nio_rt::io::Interest): Send & Sync & !Unpin);
// async_assert_fn!(nio_rt::net::TcpStream::writable(_): Send & Sync & !Unpin);

// Wasi does not support UDP
cfg_not_wasi! {
    mod udp_socket {
        use super::*;
        assert_value!(nio_rt::net::UdpSocket: !Send & !Sync & Unpin);
        async_assert_fn!(nio_rt::net::UdpSocket::bind(SocketAddr): !Send & !Sync & Unpin);
        async_assert_fn!(nio_rt::net::UdpSocket::connect(_, SocketAddr): Send & Sync & Unpin);
        async_assert_fn!(nio_rt::net::UdpSocket::peek_from(_, &mut [u8]): !Send & !Sync & Unpin);
        // async_assert_fn!(nio_rt::net::UdpSocket::readable(_): Send & Sync & !Unpin);
        // async_assert_fn!(nio_rt::net::UdpSocket::ready(_, nio_rt::io::Interest): Send & Sync & !Unpin);
        async_assert_fn!(nio_rt::net::UdpSocket::recv(_, &mut [u8]): !Send & !Sync & Unpin);
        async_assert_fn!(nio_rt::net::UdpSocket::recv_from(_, &mut [u8]): !Send & !Sync & Unpin);
        async_assert_fn!(nio_rt::net::UdpSocket::send(_, &[u8]): !Send & !Sync & Unpin);
        async_assert_fn!(nio_rt::net::UdpSocket::send_to(_, &[u8], SocketAddr): !Send & !Sync & Unpin);
        // async_assert_fn!(nio_rt::net::UdpSocket::writable(_): Send & Sync & !Unpin);
    }
}
// async_assert_fn!(tokio::net::lookup_host(SocketAddr): Send & Sync & !Unpin);
async_assert_fn!(nio_rt::net::TcpReader::peek(_, &mut [u8]): !Send & !Sync & Unpin);

assert_value!(nio_rt::JoinHandle<NN>: !Send & !Sync & Unpin);
assert_value!(nio_rt::JoinHandle<YN>: Send & Sync & Unpin);
assert_value!(nio_rt::JoinHandle<YY>: Send & Sync & Unpin);

assert_value!(nio_rt::RuntimeBuilder: Send & Sync & Unpin);
assert_value!(nio_rt::RuntimeContext: Send & Sync & Unpin);
assert_value!(nio_rt::Runtime: Send & Sync & Unpin);

assert_value!(nio_rt::Interval: !Send & !Sync & Unpin);
assert_value!(nio_rt::Sleep: !Send & !Sync & Unpin);
assert_value!(nio_rt::Timeout<BoxFutureSync<()>>: !Send & !Sync & Unpin);
assert_value!(nio_rt::Timeout<BoxFutureSend<()>>: !Send & !Sync & Unpin);
assert_value!(nio_rt::Timeout<BoxFuture<()>>: !Send & !Sync & Unpin);
async_assert_fn!(nio_rt::sleep(Duration): !Send & !Sync & Unpin);
async_assert_fn!(nio_rt::Sleep::at(Instant): !Send & !Sync & Unpin);
async_assert_fn!(nio_rt::timeout(Duration, BoxFutureSync<()>): !Send & !Sync & Unpin);
async_assert_fn!(nio_rt::timeout(Duration, BoxFutureSend<()>): !Send & !Sync & Unpin);
async_assert_fn!(nio_rt::timeout(Duration, BoxFuture<()>): !Send & !Sync & Unpin);
// async_assert_fn!(nio_rt::Timeout::at(Instant, BoxFutureSync<()>): !Send & !Sync & !Unpin);
// async_assert_fn!(nio_rt::Timeout::at(Instant, BoxFutureSend<()>): !Send & !Sync & Unpin);
// async_assert_fn!(nio_rt::Timeout::at(Instant, BoxFuture<()>): !Send & !Sync & !Unpin);
async_assert_fn!(nio_rt::Interval::tick(_): !Send & !Sync & Unpin);

assert_value!(nio_rt::LocalContext: !Send & !Sync & Unpin);

// #[cfg(unix)]
// mod unix_datagram {
//     use super::*;
//     use tokio::net::*;
//     assert_value!(UnixDatagram: Send & Sync & Unpin);
//     assert_value!(UnixListener: Send & Sync & Unpin);
//     assert_value!(UnixStream: Send & Sync & Unpin);
//     assert_value!(unix::OwnedReadHalf: Send & Sync & Unpin);
//     assert_value!(unix::OwnedWriteHalf: Send & Sync & Unpin);
//     assert_value!(unix::ReadHalf<'_>: Send & Sync & Unpin);
//     assert_value!(unix::ReuniteError: Send & Sync & Unpin);
//     assert_value!(unix::SocketAddr: Send & Sync & Unpin);
//     assert_value!(unix::UCred: Send & Sync & Unpin);
//     assert_value!(unix::WriteHalf<'_>: Send & Sync & Unpin);
//     async_assert_fn!(UnixDatagram::readable(_): Send & Sync & !Unpin);
//     async_assert_fn!(UnixDatagram::ready(_, tokio::io::Interest): Send & Sync & !Unpin);
//     async_assert_fn!(UnixDatagram::recv(_, &mut [u8]): Send & Sync & !Unpin);
//     async_assert_fn!(UnixDatagram::recv_from(_, &mut [u8]): Send & Sync & !Unpin);
//     async_assert_fn!(UnixDatagram::send(_, &[u8]): Send & Sync & !Unpin);
//     async_assert_fn!(UnixDatagram::send_to(_, &[u8], &str): Send & Sync & !Unpin);
//     async_assert_fn!(UnixDatagram::writable(_): Send & Sync & !Unpin);
//     async_assert_fn!(UnixListener::accept(_): Send & Sync & !Unpin);
//     async_assert_fn!(UnixStream::connect(&str): Send & Sync & !Unpin);
//     async_assert_fn!(UnixStream::readable(_): Send & Sync & !Unpin);
//     async_assert_fn!(UnixStream::ready(_, tokio::io::Interest): Send & Sync & !Unpin);
//     async_assert_fn!(UnixStream::writable(_): Send & Sync & !Unpin);
// }

// #[cfg(unix)]
// mod unix_pipe {
//     use super::*;
//     use tokio::net::unix::pipe::*;
//     assert_value!(OpenOptions: Send & Sync & Unpin);
//     assert_value!(Receiver: Send & Sync & Unpin);
//     assert_value!(Sender: Send & Sync & Unpin);
//     async_assert_fn!(Receiver::readable(_): Send & Sync & !Unpin);
//     async_assert_fn!(Receiver::ready(_, tokio::io::Interest): Send & Sync & !Unpin);
//     async_assert_fn!(Sender::ready(_, tokio::io::Interest): Send & Sync & !Unpin);
//     async_assert_fn!(Sender::writable(_): Send & Sync & !Unpin);
// }

// #[cfg(windows)]
// mod windows_named_pipe {
//     use super::*;
//     use tokio::net::windows::named_pipe::*;
//     assert_value!(ClientOptions: Send & Sync & Unpin);
//     assert_value!(NamedPipeClient: Send & Sync & Unpin);
//     assert_value!(NamedPipeServer: Send & Sync & Unpin);
//     assert_value!(PipeEnd: Send & Sync & Unpin);
//     assert_value!(PipeInfo: Send & Sync & Unpin);
//     assert_value!(PipeMode: Send & Sync & Unpin);
//     assert_value!(ServerOptions: Send & Sync & Unpin);
//     async_assert_fn!(NamedPipeClient::readable(_): Send & Sync & !Unpin);
//     async_assert_fn!(NamedPipeClient::ready(_, tokio::io::Interest): Send & Sync & !Unpin);
//     async_assert_fn!(NamedPipeClient::writable(_): Send & Sync & !Unpin);
//     async_assert_fn!(NamedPipeServer::connect(_): Send & Sync & !Unpin);
//     async_assert_fn!(NamedPipeServer::readable(_): Send & Sync & !Unpin);
//     async_assert_fn!(NamedPipeServer::ready(_, tokio::io::Interest): Send & Sync & !Unpin);
//     async_assert_fn!(NamedPipeServer::writable(_): Send & Sync & !Unpin);
// }

// cfg_not_wasi! {
//     mod test_process {
//         use super::*;
//         assert_value!(tokio::process::Child: Send & Sync & Unpin);
//         assert_value!(tokio::process::ChildStderr: Send & Sync & Unpin);
//         assert_value!(tokio::process::ChildStdin: Send & Sync & Unpin);
//         assert_value!(tokio::process::ChildStdout: Send & Sync & Unpin);
//         assert_value!(tokio::process::Command: Send & Sync & Unpin);
//         async_assert_fn!(tokio::process::Child::kill(_): Send & Sync & !Unpin);
//         async_assert_fn!(tokio::process::Child::wait(_): Send & Sync & !Unpin);
//         async_assert_fn!(tokio::process::Child::wait_with_output(_): Send & Sync & !Unpin);
//     }

//     async_assert_fn!(tokio::signal::ctrl_c(): Send & Sync & !Unpin);
// }

// #[cfg(unix)]
// mod unix_signal {
//     use super::*;
//     assert_value!(tokio::signal::unix::Signal: Send & Sync & Unpin);
//     assert_value!(tokio::signal::unix::SignalKind: Send & Sync & Unpin);
//     async_assert_fn!(tokio::signal::unix::Signal::recv(_): Send & Sync & !Unpin);
// }
// #[cfg(windows)]
// mod windows_signal {
//     use super::*;
//     assert_value!(tokio::signal::windows::CtrlC: Send & Sync & Unpin);
//     assert_value!(tokio::signal::windows::CtrlBreak: Send & Sync & Unpin);
//     async_assert_fn!(tokio::signal::windows::CtrlC::recv(_): Send & Sync & !Unpin);
//     async_assert_fn!(tokio::signal::windows::CtrlBreak::recv(_): Send & Sync & !Unpin);
// }
