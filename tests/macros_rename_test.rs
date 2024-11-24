// #![cfg(all(feature = "full", not(target_os = "wasi")))] // Wasi doesn't support threading

// #[allow(unused_imports)]
// use std as nio;

// use ::nio as nio1;

// async fn compute() -> usize {
//     let join = nio1::spawn(async { 1 });
//     join.await.unwrap()
// }

// #[test]
// fn crate_rename_main() {
//     assert_eq!(1, compute_main());
// }

// mod test {
//     pub use ::nio;
// }

// #[nio1::main(crate = "nio1")]
// async fn compute_main() -> usize {
//     compute().await
// }

// #[nio1::test(crate = "nio1")]
// async fn crate_rename_test() {
//     assert_eq!(1, compute().await);
// }

// #[test::nio::test(crate = "test::nio")]
// async fn crate_path_test() {
//     assert_eq!(1, compute().await);
// }
