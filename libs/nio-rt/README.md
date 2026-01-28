[![Documentation](https://docs.rs/nio/badge.svg)](https://docs.rs/nio)
[![Crates.io](https://img.shields.io/crates/v/nio.svg)](https://crates.io/crates/nio)
[![CI](https://github.com/nurmohammed840/nio/actions/workflows/ci.yml/badge.svg)](https://github.com/nurmohammed840/nio/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/license/apache-2-0)

## Nio

Nio is an async runtime for Rust.

## Task spawning APIs

Nio uses multiple worker threads to execute tasks.

| Function               |   `Send` requirement    | Thread affinity                                                     |
| ---------------------- | :---------------------: | ------------------------------------------------------------------- |
| `nio::spawn_local`     |    `!Send`  allowed     | Pinned to current thread                                            |
| `nio::spawn_pinned`    | Only captured variables | Pinned to one worker thread (selected by the runtime based on load) |
| `nio::spawn_pinned_at` | Only captured variables | Pinned to a specific worker thread (by index)                       |
| `nio::spawn`           |     `Send` required     | Not pinned, may move between threads at `.await` points             |


## Example

```toml
[dependencies]
nio = { version = "0.1", features = ["tokio-io"] }
```

By default, Nio implements async traits from [futures-io](https://docs.rs/futures-io/latest/futures_io/). But the optional "tokio-io" feature implements async traits from [tokio::io](https://docs.rs/tokio/latest/tokio/io/).

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
            // Accept the connection on different worker thread
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