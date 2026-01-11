#![cfg(not(miri))]

use futures::{Stream, StreamExt};
use nio_future::yield_now;
use nio_rt::{
    net::{TcpListener, TcpStream},
    spawn_local, test,
};

use std::io;
use std::{
    net::{IpAddr, SocketAddr},
    task::ready,
};

mod support {
    pub mod futures;
}
use support::futures::sync::{mpsc, oneshot};
use support::futures::test::assert_ok;

macro_rules! test_accept {
    ($(($ident:ident, $target:expr),)*) => {
        $(
            #[test]
            async fn $ident() {
                let mut listener = assert_ok!(TcpListener::bind($target).await);
                let addr = listener.local_addr().unwrap();

                let (tx, rx) = oneshot::channel();

                spawn_local(async move {
                    let conn = assert_ok!(listener.accept().await);
                    let socket = assert_ok!(conn.connect().await);
                    assert_ok!(tx.send(socket));
                });

                let cli = assert_ok!(TcpStream::connect(&addr).await);
                let srv = assert_ok!(rx.await);

                assert_eq!(cli.local_addr().unwrap(), srv.peer_addr().unwrap());
            }
        )*
    }
}

test_accept! {
    (ip_str, "127.0.0.1:0"),
    (host_str, "localhost:0"),
    (socket_addr, "127.0.0.1:0".parse::<SocketAddr>().unwrap()),
    (str_port_tuple, ("127.0.0.1", 0)),
    (ip_port_tuple, ("127.0.0.1".parse::<IpAddr>().unwrap(), 0)),
}

use std::pin::Pin;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering::SeqCst},
};
use std::task::{Context, Poll};

struct TrackPolls<'a> {
    npolls: Arc<AtomicUsize>,
    listener: &'a mut TcpListener,
}

impl<'a> Stream for TrackPolls<'a> {
    type Item = io::Result<(TcpStream, SocketAddr)>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.npolls.fetch_add(1, SeqCst);

        let conn = ready!(Pin::new(&mut self.listener.accept()).poll(cx)?);
        let addr = conn.peer_addr()?;
        let stream = ready!(Pin::new(&mut conn.connect()).poll(cx)?);

        Poll::Ready(Some(Ok((stream, addr))))
    }
}

#[test]
async fn no_extra_poll() {
    let mut listener = assert_ok!(TcpListener::bind("127.0.0.1:0").await);
    let addr = listener.local_addr().unwrap();

    let (tx, rx) = oneshot::channel();
    let (accepted_tx, mut accepted_rx) = mpsc::unbounded_channel();

    spawn_local(async move {
        let mut incoming = TrackPolls {
            npolls: Arc::new(AtomicUsize::new(0)),
            listener: &mut listener,
        };
        assert_ok!(tx.send(Arc::clone(&incoming.npolls)));
        while incoming.next().await.is_some() {
            accepted_tx.send(()).unwrap();
        }
    });

    let npolls = assert_ok!(rx.await);
    yield_now().await;

    // should have been polled exactly once: the initial poll
    assert_eq!(npolls.load(SeqCst), 1);

    let _ = assert_ok!(TcpStream::connect(&addr).await);
    accepted_rx.recv().await.unwrap();

    // should have been polled twice more: once to yield Some(), then once to yield Pending
    assert_eq!(npolls.load(SeqCst), 1 + 2);
}
