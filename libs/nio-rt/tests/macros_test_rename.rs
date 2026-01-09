#![cfg(not(target_os = "wasi"))] // Wasi doesn't support threading

#[allow(unused_imports)]
use std as tokio;

use ::nio_rt as nio_rt1;

mod test {
    pub use ::nio_rt;
}

async fn compute() -> usize {
    let join = nio_rt1::spawn(async { 1 });
    join.await.unwrap()
}

#[nio_rt1::main(crate = nio_rt1)]
async fn compute_main() -> usize {
    compute().await
}

#[test]
fn crate_rename_main() {
    assert_eq!(1, compute_main());
}

#[nio_rt1::test(crate = nio_rt1)]
async fn crate_rename_test() {
    assert_eq!(1, compute().await);
}

#[test::nio_rt::test(crate = test::nio_rt)]
async fn crate_path_test() {
    assert_eq!(1, compute().await);
}
