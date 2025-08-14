use nio::net::TcpListener;
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[nio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("{listener:#?}");

    loop {
        let (mut stream, addr) = listener.accept().await?;
        println!("[INCOMING] {addr:?}");

        nio::spawn(async move {
            let mut buf = vec![0; 1024];
            while let Ok(n) = stream.read(&mut buf).await {
                stream.write(&buf[..n]).await.unwrap();
            }
        });
    }
}
