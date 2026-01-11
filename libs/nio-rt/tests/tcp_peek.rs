#![cfg(not(miri))]

use std::io::Result;
use std::thread;
use std::{io::Write, net};

use nio_rt::{net::TcpStream, test};

#[test]
async fn peek() -> Result<()> {
    let listener = net::TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let t = thread::spawn(move || listener.accept());

    let left = net::TcpStream::connect(addr)?;
    left.set_nonblocking(true)?;

    let mut right = t.join().unwrap()?.0;
    right.set_nonblocking(true)?;

    let _ = right.write(&[1, 2, 3, 4])?;

    let mut left: TcpStream = left.try_into().unwrap();
    let mut buf = [0u8; 16];
    let n = left.peek(&mut buf).await?;
    assert_eq!([1, 2, 3, 4], buf[..n]);

    let n = left.read(&mut buf).await?;
    assert_eq!([1, 2, 3, 4], buf[..n]);
    Ok(())
}
