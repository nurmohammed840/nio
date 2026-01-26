[![Documentation](https://docs.rs/nio/badge.svg)](https://docs.rs/nio)
[![Crates.io](https://img.shields.io/crates/v/nio.svg)](https://crates.io/crates/nio)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/license/apache-2-0)

## Nio

Nio is async runtime for Rust.

## Example

```toml
[dependencies]
nio = "0.1"
```

Here is a basic echo server example:

```rust, no_run
use futures::AsyncWriteExt;
use nio::net::TcpListener;
use std::io::Result;

#[nio::main]
async fn main() -> Result<()> {
    let mut listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("{listener:#?}");

    loop {
        let conn = listener.accept().await?;
        println!("[INCOMING] {:?}", conn.peer_addr());

        let accept = || async {
            let mut stream = conn.connect().await?;
            let mut buf = vec![0; 1024];
            while let Ok(n) = stream.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                stream.write_all(&buf[..n]).await.unwrap();
            }
            Result::Ok(())
        };
        nio::spawn_pinned(accept);
    }
}
```