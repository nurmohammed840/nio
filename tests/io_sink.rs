#![warn(rust_2018_idioms)]
#![cfg(feature = "full")]

// use tokio::{
//     io::{self, AsyncWriteExt},
//     select,
// };

// #[nio::test]
// async fn sink_poll_write_is_cooperative() {
//     select! {
//         biased;
//         _ = async {
//             loop {
//                 let buf = vec![1, 2, 3];
//                 io::sink().write_all(&buf).await.unwrap();
//             }
//         } => {},
//         _ = nio::task::yield_now() => {}
//     }
// }

// #[nio::test]
// async fn sink_poll_flush_is_cooperative() {
//     select! {
//         biased;
//         _ = async {
//             loop {
//                 io::sink().flush().await.unwrap();
//             }
//         } => {},
//         _ = nio::task::yield_now() => {}
//     }
// }

// #[nio::test]
// async fn sink_poll_shutdown_is_cooperative() {
//     select! {
//         biased;
//         _ = async {
//             loop {
//                 io::sink().shutdown().await.unwrap();
//             }
//         } => {},
//         _ = nio::task::yield_now() => {}
//     }
// }
