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
| contention/10   | 1.898 ms  | 1.834 ms  |
| contention/100  | 4.501 ms  | 4.935 ms  |
| contention/500  | 15.574 ms | 15.804 ms |
| contention/1000 | 30.954 ms | 29.251 ms |


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

| Test Name          |    Nio    |   Tokio   |
| ------------------ | :-------: | :-------: |
| notify_one/10      | 121.73 µs | 91.541 µs |
| notify_one/50      | 204.25 µs | 90.457 µs |
| notify_one/100     | 192.81 µs | 87.222 µs |
| notify_one/200     | 202.02 µs | 86.915 µs |
| notify_one/500     | 198.40 µs | 85.629 µs |
| notify_waiters/10  | 136.45 µs | 128.23 µs |
| notify_waiters/50  | 198.11 µs | 125.44 µs |
| notify_waiters/100 | 215.11 µs | 124.70 µs |
| notify_waiters/200 | 263.39 µs | 125.58 µs |
| notify_waiters/500 | 329.58 µs | 125.25 µs |

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
