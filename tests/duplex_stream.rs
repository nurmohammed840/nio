#![warn(rust_2018_idioms)]
#![cfg(feature = "full")]

use nio_io::{AsyncReadExt, AsyncWriteExt};
use std::io::IoSlice;
use tokio::io as nio_io;

const HELLO: &[u8] = b"hello world...";

#[nio::test]
async fn write_vectored() {
    let (mut client, mut server) = nio_io::duplex(64);

    let ret = client
        .write_vectored(&[IoSlice::new(HELLO), IoSlice::new(HELLO)])
        .await
        .unwrap();
    assert_eq!(ret, HELLO.len() * 2);

    client.flush().await.unwrap();
    drop(client);

    let mut buf = Vec::with_capacity(HELLO.len() * 2);
    let bytes_read = server.read_to_end(&mut buf).await.unwrap();

    assert_eq!(bytes_read, HELLO.len() * 2);
    assert_eq!(buf, [HELLO, HELLO].concat());
}

#[nio::test]
async fn write_vectored_and_shutdown() {
    let (mut client, mut server) = nio_io::duplex(64);

    let ret = client
        .write_vectored(&[IoSlice::new(HELLO), IoSlice::new(HELLO)])
        .await
        .unwrap();
    assert_eq!(ret, HELLO.len() * 2);

    client.shutdown().await.unwrap();
    drop(client);

    let mut buf = Vec::with_capacity(HELLO.len() * 2);
    let bytes_read = server.read_to_end(&mut buf).await.unwrap();

    assert_eq!(bytes_read, HELLO.len() * 2);
    assert_eq!(buf, [HELLO, HELLO].concat());
}
