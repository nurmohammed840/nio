use std::{
    io,
    net::{SocketAddr, ToSocketAddrs},
};

pub fn bind<A, F, T>(addr: A, mut f: F) -> io::Result<T>
where
    A: ToSocketAddrs,
    F: FnMut(SocketAddr) -> io::Result<T>,
{
    let addrs = addr.to_socket_addrs()?;
    let mut last_err = None;
    for addr in addrs {
        match f(addr) {
            Err(err) => last_err = Some(err),
            result => return result,
        }
    }
    Err(last_err.unwrap_or_else(invalid_addr))
}

fn invalid_addr() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "could not resolve to any address",
    )
}
