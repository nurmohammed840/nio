#![cfg(not(target_os = "wasi"))]
#![cfg(not(miri))]

use std::{
    io::{self, Result},
    net::Shutdown,
    time::Duration,
};

use futures_lite::{AsyncReadExt, AsyncWriteExt, future};
use nio::{
    net::{TcpListener, TcpReader, TcpStream, TcpWriter, UdpSocket},
    sleep, spawn_local, test,
};

const LOREM_IPSUM: &[u8] = b"
Lorem ipsum dolor sit amet, consectetur adipiscing elit.
Donec pretium ante erat, vitae sodales mi varius quis.
Etiam vestibulum lorem vel urna tempor, eu fermentum odio aliquam.
Aliquam consequat urna vitae ipsum pulvinar, in blandit purus eleifend.
";

async fn tcp_streams() -> Result<(TcpStream, TcpStream)> {
    let mut listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let server = spawn_local(async move {
        let conn = listener.accept().await.unwrap();
        conn.connect().await.unwrap()
    });

    let client = TcpStream::connect(addr).await?;
    let server = server.await?;

    Ok((server, client))
}

#[test]
async fn tcp_connect() -> Result<()> {
    let (server, client) = tcp_streams().await?;

    assert_eq!(server.peer_addr()?, client.local_addr()?);
    assert_eq!(client.peer_addr()?, server.local_addr()?);

    // Now that the listener is closed, connect should fail.
    let err = TcpStream::connect(server.local_addr()?).await.unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::ConnectionRefused);
    Ok(())
}

#[test]
async fn tcp_peek_read() -> Result<()> {
    let (mut server, mut client) = tcp_streams().await?;

    client.write_all(LOREM_IPSUM).await?;

    let mut buf = [0; 1024];
    let n = server.peek(&mut buf).await?;
    assert_eq!(&buf[..n], LOREM_IPSUM);

    let n = server.read(&mut buf).await?;
    assert_eq!(&buf[..n], LOREM_IPSUM);
    Ok(())
}

#[test]
async fn tcp_reader_hangup() -> Result<()> {
    let (server, mut client) = tcp_streams().await?;

    let task = spawn_local(async move {
        sleep(Duration::from_secs(1)).await;
        drop(server);
    });

    while client.write_all(LOREM_IPSUM).await.is_ok() {}
    task.await?;

    Ok(())
}

#[test]
async fn tcp_writer_hangup() -> Result<()> {
    let (server, mut client) = tcp_streams().await?;

    let task = spawn_local(async move {
        sleep(Duration::from_secs(1)).await;
        drop(server);
    });

    let mut v = vec![];
    client.read_to_end(&mut v).await?;
    assert!(v.is_empty());

    task.await?;
    Ok(())
}

#[test]
async fn udp_send_recv() -> Result<()> {
    let mut socket1 = UdpSocket::bind("127.0.0.1:0").await?;
    let mut socket2 = UdpSocket::bind("127.0.0.1:0").await?;

    socket1.connect(socket2.local_addr()?).await?;

    let mut buf = [0u8; 1024];
    socket1.send(LOREM_IPSUM).await?;
    let n = socket2.peek(&mut buf).await?;
    assert_eq!(&buf[..n], LOREM_IPSUM);
    let n = socket2.recv(&mut buf).await?;
    assert_eq!(&buf[..n], LOREM_IPSUM);

    socket2.send_to(LOREM_IPSUM, socket1.local_addr()?).await?;

    let n = socket1.peek_from(&mut buf).await?.0;
    assert_eq!(&buf[..n], LOREM_IPSUM);
    let n = socket1.recv_from(&mut buf).await?.0;
    assert_eq!(&buf[..n], LOREM_IPSUM);
    Ok(())
}

// Test that we correctly re-register interests after we've previously been
// interested in both readable and writable events and then we get only one of
// those (we need to re-register interest on the other).
#[test]
async fn tcp_duplex() -> Result<()> {
    let (server, client) = tcp_streams().await?;

    async fn do_read(mut s: TcpReader) -> io::Result<()> {
        let mut buf = vec![0u8; 4096];
        loop {
            let len = s.read(&mut buf).await?;
            if len == 0 {
                return Ok(());
            }
        }
    }

    async fn do_write(mut s: TcpWriter) -> io::Result<()> {
        let buf = vec![0u8; 4096];
        for _ in 0..4096 {
            s.write_all(&buf).await?;
        }
        Ok(())
    }

    // Read from and write to `client`.
    let (reader, writer) = client.split();
    let r1 = spawn_local(do_read(reader));
    let w1 = spawn_local(do_write(writer));

    // Sleep a bit, so that reading and writing are both blocked.
    sleep(Duration::from_millis(5)).await;

    let (reader, writer) = server.split();
    // Start reading `server`, make `client` writable.
    let r2 = spawn_local(do_read(reader));

    // Finish writing to `client`.
    w1.await??;
    r2.await??;

    // Start writing to `server`, make `client` readable.
    let w2 = spawn_local(do_write(writer));

    r1.await??;
    w2.await??;
    Ok(())
}

#[test]
async fn shutdown() -> Result<()> {
    let (mut server, client) = tcp_streams().await?;
    // The writer must be closed in order for `read_to_end()` to finish.
    let mut buf = Vec::new();

    future::try_zip(server.read_to_end(&mut buf), async {
        client.shutdown(Shutdown::Write)
    })
    .await?;

    // })
    Ok(())
}

#[test]
async fn tcp_read_write() -> Result<()> {
    let (mut server, mut client) = tcp_streams().await?;

    client.write_all(LOREM_IPSUM).await?;
    client.shutdown(Shutdown::Write)?;

    let mut buffer = vec![0; LOREM_IPSUM.len()];
    server.read_exact(&mut buffer).await?;
    assert_eq!(buffer, LOREM_IPSUM);
    Ok(())
}
