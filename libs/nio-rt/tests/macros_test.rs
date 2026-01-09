#![cfg(not(target_os = "wasi"))] // Wasi doesn't support threading

use std::fmt::Debug;

use nio_rt::test;

#[test]
async fn test_macro_can_be_used_via_use() {
    nio_rt::spawn(async {}).await.unwrap();
}

#[nio_rt::test]
async fn test_macro_is_resilient_to_shadowing() {
    nio_rt::spawn(async {}).await.unwrap();
    with_arg(42);
}

// https://github.com/tokio-rs/tokio/issues/3403
#[rustfmt::skip] // this `rustfmt::skip` is necessary because unused_braces does not warn if the block contains newline.
#[nio_rt::main]
pub async fn unused_braces_main() { println!("hello") }
#[rustfmt::skip] // this `rustfmt::skip` is necessary because unused_braces does not warn if the block contains newline.
#[nio_rt::test]
async fn unused_braces_test() { assert_eq!(1 + 1, 2); }

// https://github.com/tokio-rs/tokio/pull/3766#issuecomment-835508651
#[std::prelude::v1::test]
fn trait_method() {
    trait A {
        fn f(self);

        fn g(self);
    }
    impl A for () {
        #[nio_rt::main]
        async fn f(self) {
            self.g()
        }

        fn g(self) {}
    }
    ().f()
}

// https://github.com/tokio-rs/tokio/issues/4175
#[nio_rt::main]
pub async fn issue_4175_main_1() -> ! {
    panic!();
}
#[nio_rt::main]
pub async fn issue_4175_main_2() -> std::io::Result<()> {
    panic!();
}
#[allow(unreachable_code)]
#[nio_rt::test]
pub async fn issue_4175_test() -> std::io::Result<()> {
    return Ok(());
    panic!();
}

#[nio_rt::main]
pub async fn with_arg<T>(num: T)
where
    T: PartialEq<i32> + Debug + Send + 'static,
{
    assert_eq!(num, 42);
}

// https://github.com/tokio-rs/tokio/issues/4175
#[allow(clippy::let_unit_value)]
pub mod clippy_semicolon_if_nothing_returned {
    #![deny(clippy::semicolon_if_nothing_returned)]

    #[nio_rt::main]
    pub async fn local() {
        let _x = ();
    }
    #[nio_rt::main]
    pub async fn item() {
        fn _f() {}
    }
    #[nio_rt::main]
    pub async fn semi() {
        panic!();
    }
    #[nio_rt::main]
    pub async fn empty() {
        // To trigger clippy::semicolon_if_nothing_returned lint, the block needs to contain newline.
    }
}

// https://github.com/tokio-rs/tokio/issues/5243
pub mod issue_5243 {
    macro_rules! mac {
        (async fn $name:ident() $b:block) => {
            #[::tokio::test]
            async fn $name() {
                $b
            }
        };
    }
    mac!(
        async fn foo() {}
    );
}
