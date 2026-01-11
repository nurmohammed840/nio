use nio_future::yield_now;
use nio_rt::{TaskId, spawn, spawn_local, test};

async fn task_id() -> TaskId {
    let id = async { nio_rt::task_id().await };
    yield_now().await;
    let id = id.await;
    yield_now().await;
    assert_eq!(id, nio_rt::task_id().await);
    id
}

#[test]
async fn task_id_local() {
    let jh = spawn_local(async { task_id().await });
    let id1 = jh.abort_handle().id();
    let id2 = jh.id();

    yield_now().await;
    let id3 = jh.await.unwrap();

    assert_eq!(id1, id2);
    assert_eq!(id1, id3);
    assert_eq!(id2, id3);
}

#[test]
async fn task_id_spawn() {
    let jh = spawn(async { task_id().await });
    let id1 = jh.abort_handle().id();
    let id2 = jh.id();

    yield_now().await;
    let id3 = jh.await.unwrap();

    assert_eq!(id1, id2);
    assert_eq!(id1, id3);
    assert_eq!(id2, id3);
}

#[test]
async fn task_id_collision_local() {
    let h1 = spawn_local(async { task_id().await });
    let h2 = spawn_local(async { task_id().await });

    let (id1, id2) = futures::join!(h1, h2);
    assert_ne!(id1.unwrap(), id2.unwrap());
}

#[test]
async fn task_id_collision_spawn() {
    let h1 = spawn(async { task_id().await });
    let h2 = spawn(async { task_id().await });

    let (id1, id2) = futures::join!(h1, h2);
    assert_ne!(id1.unwrap(), id2.unwrap());
}
