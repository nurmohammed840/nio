# Architecture

Under the hood, [Nio](https://github.com/nurmohammed840/nio) uses a concurrent architecture called the [Actor model](https://en.wikipedia.org/wiki/Actor_model).

Nio uses multiple worker threads to execute tasks. Each worker has a shared and local queue.
The shared queue is used for:

* Spawn a task on a different thread (`nio::spawn_pinned`,  `nio::spawn`).
* In cross-thread wakeup, the shared queue is also used to pin a `!Send` task to a specific worker thread.
* If the task is spawned via [`nio::spawn`](https://docs.rs/nio/0.1.1/nio/fn.spawn.html), the runtime use shared queue to move the task to a different worker at every `.await` point.


The worker thread can access its local queue without any synchronization. However, the async API also needs to utilize the current thread runtime to avoid cross-thread wakeup. Consider this example: 

```rust
let listener = TcpListener::bind("127.0.0.1:0").await?;
loop {
  // accept connection on thread `A` 
  let mut socket = listener.accept().await?;
  nio::spawn_pinned(|| async move {
    // using tcp connection on thread `B`
    let _ = socket.read(..).await?; 
  });
}
```

We accept tcp connection on worker `A`. Therefore, every TCP readiness happened on thread `A`, but the socket is waiting on thread `B`. 

In other words, thread `A` wakes up a task that is waiting on thread `B`. This is not good, as it requires the use of shared queue synchronization. It destroys the benefit of using the so-called *“thread-per-core”* runtime in the first place.

```rust
let listener = TcpListener::bind("127.0.0.1:0").await?;
loop { 
  let mut conn = listener.accept().await?;
  nio::spawn_pinned(|| async move {
    // accept connection on thread `B` 
    let socket = conn.connect().await?;
    let _ = socket.read(..).await?;
  });
}
```

In this example, by using the `connect()` method, we bind the TCP connection on thread `B`, ensuring that all TCP readiness events occur on thread `B` instead of thread `A`.

To avoid cross-thread wakeup, Nio does not allow I/O resources (such as TCP connections or timers) to be moved between threads. In other words, I/O resources do not implement `Send`. This enables Nio to provide a highly efficient async API, as I/O resources can be implemented without atomic operations. This design really benefits the Timer implementation. Timer algorithms are inherently single-threaded. Nio timer doesn’t require any mutex lock.

A worker can be seen as an independent async runtime. In a sense, Workers do not share any state, reactor, and timer with each other.


The Nio project is broken into separate, smaller crates. So the community can benefit from it:

* [nio-task](https://crates.io/crates/nio-task): Low-level task abstraction.
* [nio-threadpool](https://crates.io/crates/nio-threadpool): Thread pool implementation for nio runtime.