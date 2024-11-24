// #![warn(rust_2018_idioms)]
// #![cfg(all(feature = "full", not(target_os = "wasi")))] // WASI does not support all fs operations

// use tokio::io as nio_io;

// use std::io::Write;
// use tempfile::NamedTempFile;
// use nio::fs::OpenOptions;
// use nio_io::AsyncReadExt;

// const HELLO: &[u8] = b"hello world...";

// #[nio::test]
// async fn open_with_open_options_and_read() {
//     let mut tempfile = NamedTempFile::new().unwrap();
//     tempfile.write_all(HELLO).unwrap();

//     let mut file = OpenOptions::new().read(true).open(tempfile).await.unwrap();

//     let mut buf = [0; 1024];
//     let n = file.read(&mut buf).await.unwrap();

//     assert_eq!(n, HELLO.len());
//     assert_eq!(&buf[..n], HELLO);
// }

// #[nio::test]
// async fn open_options_write() {
//     // TESTING HACK: use Debug output to check the stored data
//     assert!(format!("{:?}", OpenOptions::new().write(true)).contains("write: true"));
// }

// #[nio::test]
// async fn open_options_append() {
//     // TESTING HACK: use Debug output to check the stored data
//     assert!(format!("{:?}", OpenOptions::new().append(true)).contains("append: true"));
// }

// #[nio::test]
// async fn open_options_truncate() {
//     // TESTING HACK: use Debug output to check the stored data
//     assert!(format!("{:?}", OpenOptions::new().truncate(true)).contains("truncate: true"));
// }

// #[nio::test]
// async fn open_options_create() {
//     // TESTING HACK: use Debug output to check the stored data
//     assert!(format!("{:?}", OpenOptions::new().create(true)).contains("create: true"));
// }

// #[nio::test]
// async fn open_options_create_new() {
//     // TESTING HACK: use Debug output to check the stored data
//     assert!(format!("{:?}", OpenOptions::new().create_new(true)).contains("create_new: true"));
// }

// #[nio::test]
// #[cfg(unix)]
// async fn open_options_mode() {
//     let mode = format!("{:?}", OpenOptions::new().mode(0o644));
//     // TESTING HACK: use Debug output to check the stored data
//     assert!(
//         mode.contains("mode: 420 ") || mode.contains("mode: 0o000644 "),
//         "mode is: {}",
//         mode
//     );
// }

// #[nio::test]
// #[cfg(target_os = "linux")]
// async fn open_options_custom_flags_linux() {
//     // TESTING HACK: use Debug output to check the stored data
//     assert!(
//         format!("{:?}", OpenOptions::new().custom_flags(libc::O_TRUNC))
//             .contains("custom_flags: 512,")
//     );
// }

// #[nio::test]
// #[cfg(any(target_os = "freebsd", target_os = "macos"))]
// async fn open_options_custom_flags_bsd_family() {
//     // TESTING HACK: use Debug output to check the stored data
//     assert!(
//         format!("{:?}", OpenOptions::new().custom_flags(libc::O_NOFOLLOW))
//             .contains("custom_flags: 256,")
//     );
// }
