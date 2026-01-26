# Benchmarking `nio::spawn` vs `tokio::spawn`

This benchmark suite ported from [Tokio Benchmarks](https://github.com/tokio-rs/tokio/tree/master/benches).

## Run Benchmark

```sh
cd benchmarks
cargo bench --bench rt_multi_threaded
cargo bench -F tokio --bench rt_multi_threaded
```

## Benchmark Suite

- `rt_current_thread`
- `rt_multi_threaded`
- `spawn_blocking`
- `spawn`
- `sync_broadcast`
- `sync_mpsc_oneshot`
- `sync_mpsc`
- `sync_notify`
- `sync_semaphore`
- `sync_watch`
- `timer`

### Benchmark Result

My system info:

```txt
OS: Windows 11
CPU: AMD Ryzen 7 5700 (8 Cores / 16 Threads)
```

## rt_current_thread

| Test Name              |   Nio    |   Tokio   |
| ---------------------- | :------: | :-------: |
| rt_spawn_many_local    | 94.34 µs | 168.89 µs |
| spawn_many_remote_busy | 89.84 µs | 172.65 µs |


## rt_multi_threaded

| Test Name               |    Nio    |   Tokio   |
| ----------------------- | :-------: | :-------: |
| spawn_many_local        | 2.694 ms  | 2.524 ms  |
| spawn_many_remote       | 1.110 ms  | 1.481 ms  |
| spawn_many_remote_idle  | 1.598 ms  | 2.464 ms  |
| spawn_many_remote_busy1 | 1.558 ms  | 1.801 ms  |
| spawn_many_remote_busy2 | 129.42 µs | 31.277 ms |
| ping_pong               | 353.30 µs | 313.10 µs |
| yield_many              | 3.015 ms  | 3.092 ms  |
| chained_spawn           | 95.075 µs | 206.94 µs |



## spawn_blocking

| Test Name                     |   Nio    |  Tokio   |
| ----------------------------- | :------: | :------: |
| spawn_blocking/concurrency/1  | 1.94 ms  | 2.93 ms  |
| spawn_blocking/concurrency/2  | 4.24 ms  | 4.17 ms  |
| spawn_blocking/concurrency/4  | 8.48 ms  | 10.63 ms |
| spawn_blocking/concurrency/8  | 17.23 ms | 23.87 ms |
| spawn_blocking/concurrency/16 | 27.57 ms | 48.96 ms |

## spawn

| Test Name                      |    Nio    |   Tokio   |
| ------------------------------ | :-------: | :-------: |
| basic_scheduler_spawn_10       | 11.14 µs  |  3.87 µs  |
| basic_scheduler_spawn_100      | 23.05 µs  | 24.44 µs  |
| basic_scheduler_spawn_1000     | 130.80 µs | 218.63 µs |
| basic_scheduler_spawn_10000    | 1.217 ms  | 2.313 ms  |
| threaded_scheduler_spawn_10    | 22.87 µs  |  4.31 µs  |
| threaded_scheduler_spawn_100   | 23.87 µs  | 23.18 µs  |
| threaded_scheduler_spawn_1000  | 124.97 µs | 257.75 µs |
| threaded_scheduler_spawn_10000 | 1.106 ms  | 2.578 ms  |

## sync_broadcast

| Test Name       |    Nio    |   Tokio   |
| --------------- | :-------: | :-------: |
| contention/10   | 1.8978 ms | 1.8195 ms |
| contention/100  | 4.4701 ms | 5.0437 ms |
| contention/500  | 16.237 ms | 15.982 ms |
| contention/1000 | 32.512 ms | 29.558 ms |

## sync_mpsc_oneshot

| Test Name        |    Nio    |  Tokio   |
| ---------------- | :-------: | :------: |
| request_reply    | 219.53 µs | 1.994 ms |
| request_reply #2 | 230.50 µs | 3.127 ms |


## sync_mpsc

| Test Name             |    Nio    |   Tokio   |
| --------------------- | :-------: | :-------: |
| contention/unbounded  | 382.48 µs | 432.05 µs |
| uncontented/unbounded | 211.65 µs | 180.52 µs |


## sync_notify

| Test Name          | Nio (`min_tasks_per_worker = 3`) |   Tokio   |
| ------------------ | :------------------------------: | :-------: |
| notify_one/10      |            109.85 µs             | 95.904 µs |
| notify_one/50      |            166.73 µs             | 98.382 µs |
| notify_one/100     |            162.75 µs             | 99.523 µs |
| notify_one/200     |            165.19 µs             | 101.13 µs |
| notify_one/500     |            165.06 µs             | 111.58 µs |
| notify_waiters/10  |            138.60 µs             | 119.40 µs |
| notify_waiters/50  |            131.98 µs             | 118.88 µs |
| notify_waiters/100 |            120.47 µs             | 122.05 µs |
| notify_waiters/200 |            119.87 µs             | 118.65 µs |
| notify_waiters/500 |            126.29 µs             | 122.24 µs |


In this benchmark, Nio shows slightly slower performance compared to Tokio.
The irony is that Nio is “too fast”. drain task queue immediately and goto sleep, and later wake up again.

The task do nothing except waiting for notification. To make nio look good, we can add some computation so worker are kept busy,
Or increase `min_tasks_per_worker` setting to reduce the sleep/wake cycle.

| Test Name          | Nio (`min_tasks_per_worker = 10`) |   Tokio   |
| ------------------ | :-------------------------------: | :-------: |
| notify_one/10      |             49.994 µs             | 94.787 µs |
| notify_one/50      |             99.259 µs             | 96.856 µs |
| notify_one/100     |             96.827 µs             | 99.122 µs |
| notify_one/200     |             101.46 µs             | 102.59 µs |
| notify_one/500     |             94.831 µs             | 103.83 µs |
| notify_waiters/10  |             188.09 µs             | 119.98 µs |
| notify_waiters/50  |             88.836 µs             | 123.14 µs |
| notify_waiters/100 |             107.37 µs             | 120.03 µs |
| notify_waiters/200 |             119.29 µs             | 123.29 µs |
| notify_waiters/500 |             123.69 µs             | 125.97 µs |

## sync_watch

| Test Name                   |    Nio    |   Tokio   |
| --------------------------- | :-------: | :-------: |
| contention_resubscribe/10   | 1.892 ms  | 808.97 µs |
| contention_resubscribe/100  | 4.279 ms  | 3.414 ms  |
| contention_resubscribe/500  | 15.159 ms | 14.031 ms |
| contention_resubscribe/1000 | 26.800 ms | 26.483 ms |

## timer

| Test Name              |    Nio    |   Tokio   |
| ---------------------- | :-------: | :-------: |
| single_thread_timeout  | 62.405 ns | 45.661 ns |
| multi_thread_timeout-8 | 16.293 ns | 30.877 ns |
| single_thread_sleep    | 62.135 ns | 41.127 ns |
| multi_thread_sleep-8   | 16.557 ns | 32.087 ns |
