use futures_lite::future;
use nio::{spawn_blocking, test};
use std::{
    io::Result,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

#[test]
async fn sleep() -> Result<()> {
    let dur = Duration::from_secs(1);
    let start = Instant::now();

    let mut f = spawn_blocking(move || thread::sleep(dur));
    assert!(future::poll_once(&mut f).await.is_none());
    f.await?;

    assert!(start.elapsed() >= dur);
    Ok(())
}

#[test]
async fn chan() -> Result<()> {
    const N: i32 = if cfg!(miri) { 50 } else { 100_000 };

    let (s, r) = mpsc::sync_channel::<i32>(100);
    let handle = thread::spawn(move || {
        for i in 0..N {
            s.send(i).unwrap();
        }
    });
    let mut r = spawn_blocking(|| r.into_iter()).await?;

    for i in 0..N {
        assert_eq!(r.next(), Some(i));
    }
    handle.join().unwrap();
    Ok(())
}

#[test]
async fn panic() {
    let x = spawn_blocking(|| panic!("expected failure")).await;
    let panic = x.err().unwrap().into_panic();

    // Make sure it's our panic and not an unrelated one.
    let msg = if let Some(msg) = panic.downcast_ref::<&'static str>() {
        msg.to_string()
    } else {
        *panic.downcast::<String>().unwrap()
    };
    assert_eq!(msg, "expected failure");
}
