#![warn(rust_2018_idioms)]
// #![cfg(all(feature = "full", not(miri)))]

// use tokio::io::{self, AsyncReadExt};
// use tokio::select;

// #[nio::test]
// async fn repeat_poll_read_is_cooperative() {
//     select! {
//         biased;
//         _ = async {
//             loop {
//                 let mut buf = [0u8; 4096];
//                 io::repeat(0b101).read_exact(&mut buf).await.unwrap();
//             }
//         } => {},
//         _ = nio::task::yield_now() => {}
//     }
// }
