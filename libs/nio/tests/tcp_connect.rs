#![cfg(not(miri))]

use std::io::Result;

use nio::{
    net::{TcpListener, TcpStream},
    spawn_local, test,
};

use futures::try_join;

async fn accept(mut srv: TcpListener) -> Result<TcpStream> {
    let conn = srv.accept().await?;
    let addr = conn.peer_addr()?;
    let socket = conn.connect().await?;
    assert_eq!(addr, socket.peer_addr()?);
    Ok(socket)
}

#[test]
async fn connect_v4() -> Result<()> {
    let srv = TcpListener::bind("127.0.0.1:0").await?;
    let addr = srv.local_addr()?;
    assert!(addr.is_ipv4());

    let rx = spawn_local(accept(srv));
    let mine = TcpStream::connect(&addr).await?;
    let theirs = rx.await??;

    assert_eq!(mine.local_addr()?, theirs.peer_addr()?);
    assert_eq!(theirs.local_addr()?, mine.peer_addr()?);
    Ok(())
}

#[test]
async fn connect_v6() -> Result<()> {
    let srv = TcpListener::bind("[::1]:0").await?;
    let addr = srv.local_addr()?;
    assert!(addr.is_ipv6());

    let rx = spawn_local(accept(srv));
    let mine = TcpStream::connect(&addr).await?;
    let theirs = rx.await??;

    assert_eq!(mine.local_addr()?, theirs.peer_addr()?);
    assert_eq!(theirs.local_addr()?, mine.peer_addr()?);
    Ok(())
}

async fn test_connect<A>(f: fn(&mut TcpListener) -> Result<A>) -> Result<()>
where
    A: std::net::ToSocketAddrs,
{
    let mut srv = TcpListener::bind("127.0.0.1:0").await?;
    let addr = f(&mut srv)?;

    let server = async { srv.accept().await };
    let client = async { TcpStream::connect(addr).await };
    try_join!(server, client)?;
    Ok(())
}

#[test]
async fn connect_addr_ip_string() -> Result<()> {
    test_connect(|srv| {
        let addr = srv.local_addr()?;
        Ok(format!("127.0.0.1:{}", addr.port()))
    })
    .await
}

#[test]
async fn connect_addr_ip_port_tuple() -> Result<()> {
    test_connect(|srv| {
        let addr = srv.local_addr()?;
        Ok((addr.ip(), addr.port()))
    })
    .await
}

#[test]
async fn connect_addr_ip_str_port_tuple() -> Result<()> {
    test_connect(|srv| {
        let addr = srv.local_addr()?;
        Ok(("127.0.0.1", addr.port()))
    })
    .await
}
