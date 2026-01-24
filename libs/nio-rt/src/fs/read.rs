use crate::fs::asyncify;

use std::{io, path::Path};

/// # Examples
///
/// ```no_run
/// use nio::fs;
/// use std::net::SocketAddr;
///
/// #[nio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
///     let contents = fs::read("address.txt").await?;
///     let foo: SocketAddr = String::from_utf8_lossy(&contents).parse()?;
///     Ok(())
/// }
/// ```
pub async fn read(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    let path = path.as_ref().to_owned();
    asyncify(move || std::fs::read(path)).await
}
