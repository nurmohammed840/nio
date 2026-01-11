#![cfg(not(miri))]

use std::io::Result;

use futures::{AsyncWriteExt, io};
use nio_rt::{
    net::{TcpListener, TcpStream},
    spawn_local, test,
};

mod support {
    pub mod futures;
}
use support::futures::sync::oneshot;

#[test]
async fn shutdown() -> Result<()> {
    let mut srv = TcpListener::bind("127.0.0.1:0").await?;
    let addr = srv.local_addr()?;

    let connect = || async move {
        let mut stream = TcpStream::connect(addr).await?;
        AsyncWriteExt::close(&mut stream).await?;
        let mut buf = [0u8; 1];
        let n = stream.read(&mut buf).await?;
        assert_eq!(n, 0);
        Result::Ok(())
    };

    let handle = spawn_local(connect());

    let stream = srv.accept().await?.connect().await?;
    let (mut rd, mut wr) = stream.split();

    let n = io::copy(&mut rd, &mut wr).await?;
    assert_eq!(n, 0);
    AsyncWriteExt::close(&mut wr).await?;
    handle.await??;
    Ok(())
}

#[test]
async fn shutdown_multiple_calls() -> Result<()> {
    let mut srv = TcpListener::bind("127.0.0.1:0").await?;
    let addr = srv.local_addr()?;

    let (connected_tx, connected_rx) = oneshot::channel();

    let handle = spawn_local(async move {
        let mut stream = TcpStream::connect(&addr).await?;
        connected_tx.send(()).unwrap();
        AsyncWriteExt::close(&mut stream).await?;
        AsyncWriteExt::close(&mut stream).await?;
        AsyncWriteExt::close(&mut stream).await?;
        Result::Ok(())
    });

    let mut stream = srv.accept().await?.connect().await?;
    connected_rx.await.unwrap();

    AsyncWriteExt::close(&mut stream).await?;
    handle.await??;
    Ok(())
}

/*
#[test]
async fn shutdown_after_tcp_reset() -> Result<()> {
    let mut srv = TcpListener::bind("127.0.0.1:0").await?;
    let addr = srv.local_addr()?;

    let (connected_tx, connected_rx) = oneshot::channel();
    let (dropped_tx, dropped_rx) = oneshot::channel();

    let connect = || async move {
        let mut stream = TcpStream::connect(addr).await?;
        connected_tx.send(()).unwrap();
        dropped_rx.await.unwrap();
        AsyncWriteExt::close(&mut stream).await?;
        Result::Ok(())
    };
    dropped_tx.send(()).unwrap();

    let handle = spawn_local(connect());

    let stream = srv.accept().await?.connect().await?;
    // By setting linger to 0 we will trigger a TCP reset
    stream.set_zero_linger().unwrap();
    connected_rx.await.unwrap();

    drop(stream);
    dropped_tx.send(()).unwrap();

    handle.await??;
    Ok(())
}
*/
