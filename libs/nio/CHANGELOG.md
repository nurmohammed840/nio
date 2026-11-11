## [0.2.0-rc.3] - 2026-10-31

### Removed
- **Breaking:** Removed `fs` module because this functionality can be implemented externally.
- Implement `AsyncRead` and `AsyncWrite` for `&TcpStream`