## Nio

Nio is an experimental async runtime for Rust.
For more information, check out [this article](https://nurmohammed840.github.io/posts/announcing-nio/)

Nio focuses solely on providing an async runtime, It doesn't include additional utilities like. `io`, `sync`,
You'll still need to rely on libraries like `tokio` for everything else.

## Example

Add the following dependency to your `Cargo.toml`:

```toml
[dependencies]
nio = "0.0.2"
```

Here is a basic echo server example:

```rust, ignore
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
                if n == 0 { break }
                stream.write_all(&buf[..n]).await.unwrap();
            }
        });
    }
}
```