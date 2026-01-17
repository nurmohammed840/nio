mod hyper_nio_rt;

// ```bash
// ❯ cd ./example
// ❯ cargo run -r --example hyper-server
// ```
//
// run: `❯ wrk -t4 -c100 -d10 http://127.0.0.1:4000`

use http_body_util::Full;
use hyper::{body::Bytes, server::conn::http1, service::service_fn, Request, Response};
use std::{convert::Infallible, io};

fn main() -> io::Result<()> {
    #[cfg(not(feature = "tokio"))]
    return nio_hyper_example::main();
    #[cfg(feature = "tokio")]
    return tokio_hyper_example::main();
}

// An async function that consumes a request, does nothing with it and returns a
// response.
async fn hello(_: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::new(Full::new(Bytes::from("Hello, World!"))))
}

#[cfg(not(feature = "tokio"))]
mod nio_hyper_example {
    use super::*;
    use crate::hyper_nio_rt::NioIo;
    use nio::net::TcpListener;

    #[nio::main]
    pub async fn main() -> io::Result<()> {
        let mut listener = TcpListener::bind("127.0.0.1:4000").await?;
        println!("Nio: {:#?}", listener);
        loop {
            let conn = listener.accept().await?;
            nio::spawn_pinned(|| async move {
                let stream = conn.connect().await.unwrap();
                let io = NioIo::new(stream);
                if let Err(_err) = http1::Builder::new()
                    .serve_connection(io, service_fn(hello))
                    .await
                {
                    // eprintln!("Error serving connection: {_err:?}");
                }
            });
        }
    }
}

#[cfg(feature = "tokio")]
mod tokio_hyper_example {
    use super::*;
    use hyper_util::rt::TokioIo;
    use tokio::net::TcpListener;

    #[tokio::main]
    pub async fn main() -> io::Result<()> {
        tokio::spawn(run()).await?
    }

    pub async fn run() -> io::Result<()> {
        let listener = TcpListener::bind("127.0.0.1:5000").await?;
        println!("Tokio: {:#?}", listener);
        loop {
            let (stream, _) = listener.accept().await?;
            tokio::spawn(async move {
                let io = TokioIo::new(stream);
                if let Err(_err) = http1::Builder::new()
                    .serve_connection(io, service_fn(hello))
                    .await
                {
                    // eprintln!("Error serving connection: {_err:?}");
                }
            });
        }
    }
}
