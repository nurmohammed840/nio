#![warn(rust_2018_idioms)]
#![cfg(feature = "full")]
#![allow(clippy::type_complexity, clippy::diverging_sub_expression)]

use std::cell::Cell;
use std::net::SocketAddr;
use std::rc::Rc;
// use std::future::Future;
// use std::io::SeekFrom;
// use std::pin::Pin;
use nio::time::{Duration /* Instant */};

// The names of these structs behaves better when sorted.
// Send: Yes, Sync: Yes
#[derive(Clone)]
#[allow(unused)]
struct YY {}

// Send: Yes, Sync: No
#[derive(Clone)]
#[allow(unused)]
struct YN {
    _value: Cell<u8>,
}

// Send: No, Sync: No
#[derive(Clone)]
#[allow(unused)]
struct NN {
    _value: Rc<u8>,
}

#[allow(dead_code)]
type BoxFutureSync<T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + Sync>>;
#[allow(dead_code)]
type BoxFutureSend<T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send>>;
#[allow(dead_code)]
type BoxFuture<T> = std::pin::Pin<Box<dyn std::future::Future<Output = T>>>;

// #[allow(dead_code)]
// type BoxAsyncRead = std::pin::Pin<Box<dyn nio::io::AsyncBufRead + Send + Sync>>;
// #[allow(dead_code)]
// type BoxAsyncSeek = std::pin::Pin<Box<dyn nio::io::AsyncSeek + Send + Sync>>;
// #[allow(dead_code)]
// type BoxAsyncWrite = std::pin::Pin<Box<dyn nio::io::AsyncWrite + Send + Sync>>;

#[allow(dead_code)]
fn require_send<T: Send>(_t: &T) {}
#[allow(dead_code)]
fn require_sync<T: Sync>(_t: &T) {}
#[allow(dead_code)]
fn require_unpin<T: Unpin>(_t: &T) {}

#[allow(dead_code)]
struct Invalid;

#[allow(unused)]
trait AmbiguousIfSend<A> {
    fn some_item(&self) {}
}
impl<T: ?Sized> AmbiguousIfSend<()> for T {}
impl<T: ?Sized + Send> AmbiguousIfSend<Invalid> for T {}

#[allow(unused)]
trait AmbiguousIfSync<A> {
    fn some_item(&self) {}
}
impl<T: ?Sized> AmbiguousIfSync<()> for T {}
impl<T: ?Sized + Sync> AmbiguousIfSync<Invalid> for T {}

#[allow(unused)]
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

cfg_not_wasi! {
    mod fs {
        use super::*;
        assert_value!(nio::fs::DirBuilder: Send & Sync & Unpin);
        assert_value!(nio::fs::DirEntry: Send & Sync & Unpin);
        // assert_value!(nio::fs::File: Send & Sync & Unpin);
        // assert_value!(nio::fs::OpenOptions: Send & Sync & Unpin);
        assert_value!(nio::fs::ReadDir: Send & Sync & Unpin);

        async_assert_fn!(nio::fs::canonicalize(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio::fs::copy(&str, &str): Send & Sync & !Unpin);
        async_assert_fn!(nio::fs::create_dir(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio::fs::create_dir_all(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio::fs::hard_link(&str, &str): Send & Sync & !Unpin);
        async_assert_fn!(nio::fs::metadata(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio::fs::read(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio::fs::read_dir(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio::fs::read_link(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio::fs::read_to_string(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio::fs::remove_dir(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio::fs::remove_dir_all(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio::fs::remove_file(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio::fs::rename(&str, &str): Send & Sync & !Unpin);
        async_assert_fn!(nio::fs::set_permissions(&str, std::fs::Permissions): Send & Sync & !Unpin);
        async_assert_fn!(nio::fs::symlink_metadata(&str): Send & Sync & !Unpin);
        async_assert_fn!(nio::fs::write(&str, Vec<u8>): Send & Sync & !Unpin);
        async_assert_fn!(nio::fs::ReadDir::next_entry(_): Send & Sync & !Unpin);
        // async_assert_fn!(nio::fs::OpenOptions::open(_, &str): Send & Sync & !Unpin);
        async_assert_fn!(nio::fs::DirBuilder::create(_, &str): Send & Sync & !Unpin);
        async_assert_fn!(nio::fs::DirEntry::metadata(_): Send & Sync & !Unpin);
        async_assert_fn!(nio::fs::DirEntry::file_type(_): Send & Sync & !Unpin);
        // async_assert_fn!(nio::fs::File::open(&str): Send & Sync & !Unpin);
        // async_assert_fn!(nio::fs::File::create(&str): Send & Sync & !Unpin);
        // async_assert_fn!(nio::fs::File::sync_all(_): Send & Sync & !Unpin);
        // async_assert_fn!(nio::fs::File::sync_data(_): Send & Sync & !Unpin);
        // async_assert_fn!(nio::fs::File::set_len(_, u64): Send & Sync & !Unpin);
        // async_assert_fn!(nio::fs::File::metadata(_): Send & Sync & !Unpin);
        // async_assert_fn!(nio::fs::File::try_clone(_): Send & Sync & !Unpin);
        // async_assert_fn!(nio::fs::File::into_std(_): Send & Sync & !Unpin);
        // async_assert_fn!(
        //     nio::fs::File::set_permissions(_, std::fs::Permissions): Send & Sync & !Unpin
        // );
    }
}

cfg_not_wasi! {
    assert_value!(nio::net::TcpSocket: Send & Sync & Unpin);
    async_assert_fn!(nio::net::TcpListener::bind(SocketAddr): Send & Sync & !Unpin);
    async_assert_fn!(nio::net::TcpStream::connect(SocketAddr): Send & !Sync & !Unpin);
}

assert_value!(nio::net::TcpListener: Send & !Sync & Unpin);
assert_value!(nio::net::TcpStream: Send & !Sync & Unpin);
assert_value!(nio::net::tcp::OwnedReadHalf: Send & !Sync & Unpin);
assert_value!(nio::net::tcp::OwnedWriteHalf: Send & !Sync & Unpin);
assert_value!(nio::net::tcp::ReuniteError: Send & !Sync & Unpin);
assert_value!(nio::net::tcp::ReadHalf<'_>: Send & !Sync & Unpin);
assert_value!(nio::net::tcp::WriteHalf<'_>: Send & !Sync & Unpin);
async_assert_fn!(nio::net::TcpListener::accept(_): Send & Sync & Unpin);
async_assert_fn!(nio::net::TcpStream::peek(_, &mut [u8]): Send & Sync & Unpin);
async_assert_fn!(nio::net::TcpStream::readable(_): Send & Sync & Unpin);
async_assert_fn!(nio::net::TcpStream::writable(_): Send & Sync & Unpin);

// Wasi does not support UDP
cfg_not_wasi! {
    mod udp_socket {
        use super::*;
        assert_value!(nio::net::UdpSocket: Send & !Sync & Unpin);
        async_assert_fn!(nio::net::UdpSocket::bind(SocketAddr): Send & !Sync & Unpin);
        async_assert_fn!(nio::net::UdpSocket::connect(_, SocketAddr): Send & Sync & Unpin);
        async_assert_fn!(nio::net::UdpSocket::peek_from(_, &mut [u8]): Send & Sync & Unpin);
        async_assert_fn!(nio::net::UdpSocket::readable(_): Send & Sync & Unpin);
        async_assert_fn!(nio::net::UdpSocket::recv(_, &mut [u8]): Send & Sync & Unpin);
        async_assert_fn!(nio::net::UdpSocket::recv_from(_, &mut [u8]): Send & Sync & Unpin);
        async_assert_fn!(nio::net::UdpSocket::send(_, &[u8]): Send & Sync & Unpin);
        async_assert_fn!(nio::net::UdpSocket::send_to(_, &[u8], SocketAddr): Send & Sync & Unpin);
        async_assert_fn!(nio::net::UdpSocket::writable(_): Send & Sync & Unpin);
    }
}
// async_assert_fn!(nio::net::lookup_host(SocketAddr): Send & Sync & !Unpin);
async_assert_fn!(nio::net::tcp::ReadHalf::peek(_, &mut [u8]): Send & Sync & Unpin);

// #[cfg(unix)]
// mod unix_datagram {
//     use super::*;
//     use nio::net::*;
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
//     async_assert_fn!(UnixDatagram::ready(_, nio::io::Interest): Send & Sync & !Unpin);
//     async_assert_fn!(UnixDatagram::recv(_, &mut [u8]): Send & Sync & !Unpin);
//     async_assert_fn!(UnixDatagram::recv_from(_, &mut [u8]): Send & Sync & !Unpin);
//     async_assert_fn!(UnixDatagram::send(_, &[u8]): Send & Sync & !Unpin);
//     async_assert_fn!(UnixDatagram::send_to(_, &[u8], &str): Send & Sync & !Unpin);
//     async_assert_fn!(UnixDatagram::writable(_): Send & Sync & !Unpin);
//     async_assert_fn!(UnixListener::accept(_): Send & Sync & !Unpin);
//     async_assert_fn!(UnixStream::connect(&str): Send & Sync & !Unpin);
//     async_assert_fn!(UnixStream::readable(_): Send & Sync & !Unpin);
//     async_assert_fn!(UnixStream::ready(_, nio::io::Interest): Send & Sync & !Unpin);
//     async_assert_fn!(UnixStream::writable(_): Send & Sync & !Unpin);
// }

// #[cfg(unix)]
// mod unix_pipe {
//     use super::*;
//     use nio::net::unix::pipe::*;
//     assert_value!(OpenOptions: Send & Sync & Unpin);
//     assert_value!(Receiver: Send & Sync & Unpin);
//     assert_value!(Sender: Send & Sync & Unpin);
//     async_assert_fn!(Receiver::readable(_): Send & Sync & !Unpin);
//     async_assert_fn!(Receiver::ready(_, nio::io::Interest): Send & Sync & !Unpin);
//     async_assert_fn!(Sender::ready(_, nio::io::Interest): Send & Sync & !Unpin);
//     async_assert_fn!(Sender::writable(_): Send & Sync & !Unpin);
// }

// #[cfg(windows)]
// mod windows_named_pipe {
//     use super::*;
//     use nio::net::windows::named_pipe::*;
//     assert_value!(ClientOptions: Send & Sync & Unpin);
//     assert_value!(NamedPipeClient: Send & Sync & Unpin);
//     assert_value!(NamedPipeServer: Send & Sync & Unpin);
//     assert_value!(PipeEnd: Send & Sync & Unpin);
//     assert_value!(PipeInfo: Send & Sync & Unpin);
//     assert_value!(PipeMode: Send & Sync & Unpin);
//     assert_value!(ServerOptions: Send & Sync & Unpin);
//     async_assert_fn!(NamedPipeClient::readable(_): Send & Sync & !Unpin);
//     async_assert_fn!(NamedPipeClient::ready(_, nio::io::Interest): Send & Sync & !Unpin);
//     async_assert_fn!(NamedPipeClient::writable(_): Send & Sync & !Unpin);
//     async_assert_fn!(NamedPipeServer::connect(_): Send & Sync & !Unpin);
//     async_assert_fn!(NamedPipeServer::readable(_): Send & Sync & !Unpin);
//     async_assert_fn!(NamedPipeServer::ready(_, nio::io::Interest): Send & Sync & !Unpin);
//     async_assert_fn!(NamedPipeServer::writable(_): Send & Sync & !Unpin);
// }

// cfg_not_wasi! {
//     mod test_process {
//         use super::*;
//         assert_value!(nio::process::Child: Send & Sync & Unpin);
//         assert_value!(nio::process::ChildStderr: Send & Sync & Unpin);
//         assert_value!(nio::process::ChildStdin: Send & Sync & Unpin);
//         assert_value!(nio::process::ChildStdout: Send & Sync & Unpin);
//         assert_value!(nio::process::Command: Send & Sync & Unpin);
//         async_assert_fn!(nio::process::Child::kill(_): Send & Sync & !Unpin);
//         async_assert_fn!(nio::process::Child::wait(_): Send & Sync & !Unpin);
//         async_assert_fn!(nio::process::Child::wait_with_output(_): Send & Sync & !Unpin);
//     }
//     async_assert_fn!(nio::signal::ctrl_c(): Send & Sync & !Unpin);
// }

// #[cfg(unix)]
// mod unix_signal {
//     use super::*;
//     assert_value!(nio::signal::unix::Signal: Send & Sync & Unpin);
//     assert_value!(nio::signal::unix::SignalKind: Send & Sync & Unpin);
//     async_assert_fn!(nio::signal::unix::Signal::recv(_): Send & Sync & !Unpin);
// }
// #[cfg(windows)]
// mod windows_signal {
//     use super::*;
//     assert_value!(nio::signal::windows::CtrlC: Send & Sync & Unpin);
//     assert_value!(nio::signal::windows::CtrlBreak: Send & Sync & Unpin);
//     async_assert_fn!(nio::signal::windows::CtrlC::recv(_): Send & Sync & !Unpin);
//     async_assert_fn!(nio::signal::windows::CtrlBreak::recv(_): Send & Sync & !Unpin);
// }

assert_value!(nio::task::JoinError: Send & Sync & Unpin);
assert_value!(nio::task::JoinHandle<NN>: !Send & !Sync & Unpin);
assert_value!(nio::task::JoinHandle<YN>: Send & Sync & Unpin);
assert_value!(nio::task::JoinHandle<YY>: Send & Sync & Unpin);
// assert_value!(nio::task::JoinSet<NN>: !Send & !Sync & Unpin);
// assert_value!(nio::task::JoinSet<YN>: Send & Sync & Unpin);
// assert_value!(nio::task::JoinSet<YY>: Send & Sync & Unpin);
// assert_value!(nio::task::LocalSet: !Send & !Sync & Unpin);

// async_assert_fn!(nio::task::JoinSet<Cell<u32>>::join_next(_): Send & Sync & !Unpin);
// async_assert_fn!(nio::task::JoinSet<Cell<u32>>::shutdown(_): Send & Sync & !Unpin);
// async_assert_fn!(nio::task::JoinSet<Rc<u32>>::join_next(_): !Send & !Sync & !Unpin);
// async_assert_fn!(nio::task::JoinSet<Rc<u32>>::shutdown(_): !Send & !Sync & !Unpin);
// async_assert_fn!(nio::task::JoinSet<u32>::join_next(_): Send & Sync & !Unpin);
// async_assert_fn!(nio::task::JoinSet<u32>::shutdown(_): Send & Sync & !Unpin);
// async_assert_fn!(nio::task::LocalKey<Cell<u32>>::scope(_, Cell<u32>, BoxFuture<()>): !Send & !Sync & !Unpin);
// async_assert_fn!(nio::task::LocalKey<Cell<u32>>::scope(_, Cell<u32>, BoxFutureSend<()>): Send & !Sync & !Unpin);
// async_assert_fn!(nio::task::LocalKey<Cell<u32>>::scope(_, Cell<u32>, BoxFutureSync<()>): Send & !Sync & !Unpin);
// async_assert_fn!(nio::task::LocalKey<Rc<u32>>::scope(_, Rc<u32>, BoxFuture<()>): !Send & !Sync & !Unpin);
// async_assert_fn!(nio::task::LocalKey<Rc<u32>>::scope(_, Rc<u32>, BoxFutureSend<()>): !Send & !Sync & !Unpin);
// async_assert_fn!(nio::task::LocalKey<Rc<u32>>::scope(_, Rc<u32>, BoxFutureSync<()>): !Send & !Sync & !Unpin);
// async_assert_fn!(nio::task::LocalKey<u32>::scope(_, u32, BoxFuture<()>): !Send & !Sync & !Unpin);
// async_assert_fn!(nio::task::LocalKey<u32>::scope(_, u32, BoxFutureSend<()>): Send & !Sync & !Unpin);
// async_assert_fn!(nio::task::LocalKey<u32>::scope(_, u32, BoxFutureSync<()>): Send & Sync & !Unpin);
// async_assert_fn!(nio::task::LocalSet::run_until(_, BoxFutureSync<()>): !Send & !Sync & !Unpin);
// async_assert_fn!(nio::task::unconstrained(BoxFuture<()>): !Send & !Sync & Unpin);
// async_assert_fn!(nio::task::unconstrained(BoxFutureSend<()>): Send & !Sync & Unpin);
// async_assert_fn!(nio::task::unconstrained(BoxFutureSync<()>): Send & Sync & Unpin);

assert_value!(nio::runtime::Builder: Send & Sync & Unpin);
// assert_value!(nio::runtime::EnterGuard<'_>: !Send & Sync & Unpin);
assert_value!(nio::runtime::Handle: Send & Sync & Unpin);
assert_value!(nio::runtime::Runtime: Send & Sync & Unpin);

// assert_value!(nio::time::Interval: Send & Sync & Unpin);
// assert_value!(nio::time::Instant: Send & Sync & Unpin);
assert_value!(nio::time::Sleep: Send & Sync & Unpin);
assert_value!(nio::time::Timeout<BoxFutureSync<()>>: Send & Sync & Unpin);
assert_value!(nio::time::Timeout<BoxFutureSend<()>>: Send & !Sync & Unpin);
assert_value!(nio::time::Timeout<BoxFuture<()>>: !Send & !Sync & Unpin);
assert_value!(nio::time::TimeoutError: Send & Sync & Unpin);
// assert_value!(nio::time::error::Error: Send & Sync & Unpin);
// async_assert_fn!(nio::time::advance(Duration): Send & Sync & !Unpin);
async_assert_fn!(nio::time::sleep(Duration): Send & Sync & Unpin);
// async_assert_fn!(nio::time::sleep_until(Instant): Send & Sync & !Unpin);
async_assert_fn!(nio::time::timeout(Duration, BoxFutureSync<()>): Send & Sync & Unpin);
async_assert_fn!(nio::time::timeout(Duration, BoxFutureSend<()>): Send & !Sync & Unpin);
async_assert_fn!(nio::time::timeout(Duration, BoxFuture<()>): !Send & !Sync & Unpin);
// async_assert_fn!(nio::time::timeout_at(Instant, BoxFutureSync<()>): Send & Sync & !Unpin);
// async_assert_fn!(nio::time::timeout_at(Instant, BoxFutureSend<()>): Send & !Sync & !Unpin);
// async_assert_fn!(nio::time::timeout_at(Instant, BoxFuture<()>): !Send & !Sync & !Unpin);
// async_assert_fn!(nio::time::Interval::tick(_): Send & Sync & !Unpin);

// assert_value!(nio::io::BufReader<TcpStream>: Send & Sync & Unpin);
// assert_value!(nio::io::BufStream<TcpStream>: Send & Sync & Unpin);
// assert_value!(nio::io::BufWriter<TcpStream>: Send & Sync & Unpin);
// assert_value!(nio::io::DuplexStream: Send & Sync & Unpin);
// assert_value!(nio::io::Empty: Send & Sync & Unpin);
// assert_value!(nio::io::Interest: Send & Sync & Unpin);
// assert_value!(nio::io::Lines<TcpStream>: Send & Sync & Unpin);
// assert_value!(nio::io::ReadBuf<'_>: Send & Sync & Unpin);
// assert_value!(nio::io::ReadHalf<TcpStream>: Send & Sync & Unpin);
// assert_value!(nio::io::Ready: Send & Sync & Unpin);
// assert_value!(nio::io::Repeat: Send & Sync & Unpin);
// assert_value!(nio::io::Sink: Send & Sync & Unpin);
// assert_value!(nio::io::Split<TcpStream>: Send & Sync & Unpin);
// assert_value!(nio::io::Stderr: Send & Sync & Unpin);
// assert_value!(nio::io::Stdin: Send & Sync & Unpin);
// assert_value!(nio::io::Stdout: Send & Sync & Unpin);
// assert_value!(nio::io::Take<TcpStream>: Send & Sync & Unpin);
// assert_value!(nio::io::WriteHalf<TcpStream>: Send & Sync & Unpin);
// async_assert_fn!(nio::io::copy(&mut TcpStream, &mut TcpStream): Send & Sync & !Unpin);
// async_assert_fn!(
//     nio::io::copy_bidirectional(&mut TcpStream, &mut TcpStream): Send & Sync & !Unpin
// );
// async_assert_fn!(nio::io::copy_buf(&mut nio::io::BufReader<TcpStream>, &mut TcpStream): Send & Sync & !Unpin);
// async_assert_fn!(nio::io::empty(): Send & Sync & Unpin);
// async_assert_fn!(nio::io::repeat(u8): Send & Sync & Unpin);
// async_assert_fn!(nio::io::sink(): Send & Sync & Unpin);
// async_assert_fn!(nio::io::split(TcpStream): Send & Sync & Unpin);
// async_assert_fn!(nio::io::stderr(): Send & Sync & Unpin);
// async_assert_fn!(nio::io::stdin(): Send & Sync & Unpin);
// async_assert_fn!(nio::io::stdout(): Send & Sync & Unpin);
// async_assert_fn!(nio::io::Split<nio::io::BufReader<TcpStream>>::next_segment(_): Send & Sync & !Unpin);
// async_assert_fn!(nio::io::Lines<nio::io::BufReader<TcpStream>>::next_line(_): Send & Sync & !Unpin);

// #[cfg(unix)]
// mod unix_asyncfd {
//     use super::*;
//     use nio::io::unix::*;

//     #[allow(unused)]
//     struct ImplsFd<T> {
//         _t: T,
//     }
//     impl<T> std::os::unix::io::AsRawFd for ImplsFd<T> {
//         fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
//             unreachable!()
//         }
//     }

//     assert_value!(AsyncFd<ImplsFd<YY>>: Send & Sync & Unpin);
//     assert_value!(AsyncFd<ImplsFd<YN>>: Send & !Sync & Unpin);
//     assert_value!(AsyncFd<ImplsFd<NN>>: !Send & !Sync & Unpin);
//     assert_value!(AsyncFdReadyGuard<'_, ImplsFd<YY>>: Send & Sync & Unpin);
//     assert_value!(AsyncFdReadyGuard<'_, ImplsFd<YN>>: !Send & !Sync & Unpin);
//     assert_value!(AsyncFdReadyGuard<'_, ImplsFd<NN>>: !Send & !Sync & Unpin);
//     assert_value!(AsyncFdReadyMutGuard<'_, ImplsFd<YY>>: Send & Sync & Unpin);
//     assert_value!(AsyncFdReadyMutGuard<'_, ImplsFd<YN>>: Send & !Sync & Unpin);
//     assert_value!(AsyncFdReadyMutGuard<'_, ImplsFd<NN>>: !Send & !Sync & Unpin);
//     assert_value!(TryIoError: Send & Sync & Unpin);
//     async_assert_fn!(AsyncFd<ImplsFd<YY>>::readable(_): Send & Sync & !Unpin);
//     async_assert_fn!(AsyncFd<ImplsFd<YY>>::readable_mut(_): Send & Sync & !Unpin);
//     async_assert_fn!(AsyncFd<ImplsFd<YY>>::writable(_): Send & Sync & !Unpin);
//     async_assert_fn!(AsyncFd<ImplsFd<YY>>::writable_mut(_): Send & Sync & !Unpin);
//     async_assert_fn!(AsyncFd<ImplsFd<YN>>::readable(_): !Send & !Sync & !Unpin);
//     async_assert_fn!(AsyncFd<ImplsFd<YN>>::readable_mut(_): Send & !Sync & !Unpin);
//     async_assert_fn!(AsyncFd<ImplsFd<YN>>::writable(_): !Send & !Sync & !Unpin);
//     async_assert_fn!(AsyncFd<ImplsFd<YN>>::writable_mut(_): Send & !Sync & !Unpin);
//     async_assert_fn!(AsyncFd<ImplsFd<NN>>::readable(_): !Send & !Sync & !Unpin);
//     async_assert_fn!(AsyncFd<ImplsFd<NN>>::readable_mut(_): !Send & !Sync & !Unpin);
//     async_assert_fn!(AsyncFd<ImplsFd<NN>>::writable(_): !Send & !Sync & !Unpin);
//     async_assert_fn!(AsyncFd<ImplsFd<NN>>::writable_mut(_): !Send & !Sync & !Unpin);
// }
