#![cfg(not(miri))]

use std::io::Result;

use futures::{AsyncReadExt, AsyncWriteExt};
use nio::{
    net::{TcpListener, TcpStream},
    spawn_local, test,
};

#[test]
async fn echo_server() -> Result<()> {
    const ITER: usize = 1024;

    let mut srv = TcpListener::bind("127.0.0.1:0").await?;
    let addr = srv.local_addr()?;

    let msg = "foo bar baz";

    let connect = || async move {
        let mut stream = TcpStream::connect(&addr).await?;
        for _ in 0..ITER {
            // write
            stream.write_all(msg.as_bytes()).await?;

            // read
            let mut buf = [0; 11];
            stream.read_exact(&mut buf).await?;
            assert_eq!(&buf[..], msg.as_bytes());
        }
        Result::Ok(())
    };
    let rx = spawn_local(connect());
    let stream = srv.accept().await?.connect().await?;
    let (mut rd, mut wr) = stream.split();

    let n = futures::io::copy(&mut rd, &mut wr).await?;
    assert_eq!(n, (ITER * msg.len()) as u64);
    rx.await?
}
