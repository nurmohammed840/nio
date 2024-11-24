mod hyper_nio_rt;

// ```bash
// ❯ cd ./example
// ❯ cargo run -r --example hyper-server
// ```
//
// run: `❯ wrk -t4 -c100 -d10 http://127.0.0.1:4000`

// -----------------------------------------------------------
#[cfg(feature = "tokio")]
use hyper_util::rt::TokioIo as NioIo;
#[cfg(feature = "tokio")]
use tokio as nio;

#[cfg(not(feature = "tokio"))]
use hyper_nio_rt::NioIo;

// -----------------------------------------------------------

use std::convert::Infallible;
use std::net::SocketAddr;

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use nio::net::TcpListener;

// This would normally come from the `hyper-util` crate, but we can't depend
// on that here because it would be a cyclical dependency.

// An async function that consumes a request, does nothing with it and returns a
// response.
async fn hello(_: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::new(Full::new(Bytes::from("Hello, World!"))))
}

#[nio::main]
async fn main() {
    let _ = nio::spawn(run()).await;
}

async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(feature = "tokio")]
    let addr = SocketAddr::from(([127, 0, 0, 1], 5000));
    #[cfg(not(feature = "tokio"))]
    let addr = SocketAddr::from(([127, 0, 0, 1], 4000));

    // We create a TcpListener and bind it to 127.0.0.1:3000
    let listener = TcpListener::bind(addr).await?;
    println!("listener: {:#?}", listener);

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;
        // Use an adapter to access something implementing `nio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = NioIo::new(stream);
        // Spawn a nio task to serve multiple connections concurrently
        nio::spawn(async move {
            // Finally, we bind the incoming connection to our `hello` service
            if let Err(_err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(io, service_fn(hello))
                .await
            {
                // eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}
