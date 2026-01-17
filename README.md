## Nio

Nio is an experimental async runtime for Rust.
For more information, check out [this article](https://nurmohammed840.github.io/posts/announcing-nio/)

Nio focuses solely on providing an async runtime, It doesn't include additional utilities like. `io`, `sync`,
You'll still need to rely on libraries like `tokio` for everything else.

## Example

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