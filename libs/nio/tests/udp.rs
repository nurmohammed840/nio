#![cfg(not(miri))]

use nio::{net::UdpSocket, test};
use std::{future::poll_fn, io::Result, pin::Pin};

const MSG: &[u8] = b"hello";

#[test]
async fn send_recv() -> Result<()> {
    let mut sender = UdpSocket::bind("127.0.0.1:0").await?;
    let mut receiver = UdpSocket::bind("127.0.0.1:0").await?;

    sender.connect(receiver.local_addr()?).await?;
    receiver.connect(sender.local_addr()?).await?;

    sender.send(MSG).await?;

    let mut recv_buf = [0u8; 32];
    let len = receiver.recv(&mut recv_buf[..]).await?;

    assert_eq!(&recv_buf[..len], MSG);
    Ok(())
}

#[test]
async fn send_recv_poll() -> Result<()> {
    let mut sender = UdpSocket::bind("127.0.0.1:0").await?;
    let mut receiver = UdpSocket::bind("127.0.0.1:0").await?;

    sender.connect(receiver.local_addr()?).await?;
    receiver.connect(sender.local_addr()?).await?;

    poll_fn(|cx| Pin::new(&mut sender.send(MSG)).poll(cx)).await?;

    let mut recv_buf = [0u8; 32];
    let len = poll_fn(|cx| Pin::new(&mut receiver.recv(&mut recv_buf[..])).poll(cx)).await?;

    assert_eq!(&recv_buf[..len], MSG);
    Ok(())
}

#[test]
async fn send_to_recv_from() -> Result<()> {
    let mut sender = UdpSocket::bind("127.0.0.1:0").await?;
    let mut receiver = UdpSocket::bind("127.0.0.1:0").await?;

    let receiver_addr = receiver.local_addr()?;
    sender.send_to(MSG, receiver_addr).await?;

    let mut recv_buf = [0u8; 32];
    let (len, addr) = receiver.recv_from(&mut recv_buf[..]).await?;

    assert_eq!(&recv_buf[..len], MSG);
    assert_eq!(addr, sender.local_addr()?);
    Ok(())
}

#[test]
async fn send_to_peek_from() -> Result<()> {
    let mut sender = UdpSocket::bind("127.0.0.1:0").await?;
    let mut receiver = UdpSocket::bind("127.0.0.1:0").await?;

    let receiver_addr = receiver.local_addr()?;
    poll_fn(|cx| Pin::new(&mut sender.send_to(MSG, receiver_addr)).poll(cx)).await?;

    // peek
    let mut recv_buf = [0u8; 32];
    let (n, addr) = receiver.peek_from(&mut recv_buf).await?;
    assert_eq!(&recv_buf[..n], MSG);
    assert_eq!(addr, sender.local_addr()?);

    // peek
    let mut recv_buf = [0u8; 32];
    let (n, addr) = receiver.peek_from(&mut recv_buf).await?;
    assert_eq!(&recv_buf[..n], MSG);
    assert_eq!(addr, sender.local_addr()?);

    let mut recv_buf = [0u8; 32];
    let (n, addr) = receiver.recv_from(&mut recv_buf).await?;
    assert_eq!(&recv_buf[..n], MSG);
    assert_eq!(addr, sender.local_addr()?);

    Ok(())
}
