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

| Test Name               |    Nio    | Tokio |
| ----------------------- | :-------: | :---: |
| spawn_many_local        |  2.66 ms  |   —   |
| spawn_many_remote       |  1.17 ms  |   —   |
| spawn_many_remote_idle  |  1.60 ms  |   —   |
| spawn_many_remote_busy1 |  1.05 ms  |   —   |
| spawn_many_remote_busy2 |  125 µs   |   —   |
| ping_pong               | 348.54 µs |   —   |
| yield_many              |   3 ms    |   —   |
| chained_spawn           | 89.57 µs  |   —   |


## spawn_blocking

| Test Name                     |   Nio    |  Tokio   |
| ----------------------------- | :------: | :------: |
| spawn_blocking/concurrency/1  | 1.94 ms  | 2.93 ms  |
| spawn_blocking/concurrency/2  | 4.24 ms  | 4.17 ms  |
| spawn_blocking/concurrency/4  | 8.48 ms  | 10.63 ms |
| spawn_blocking/concurrency/8  | 17.23 ms | 23.87 ms |
| spawn_blocking/concurrency/16 | 27.57 ms | 48.96 ms |

## spawn

| Test Name                     |    Nio    |   Tokio   |
| ----------------------------- | :-------: | :-------: |
| basic_scheduler_spawn         |  7.77 µs  |  2.23 µs  |
| basic_scheduler_spawn_10      | 10.93 µs  |  4.07 µs  |
| basic_scheduler_spawn_100     | 22.51 µs  | 24.10 µs  |
| basic_scheduler_spawn_1000    | 130.12 µs | 221.97 µs |
| threaded_scheduler_spawn      |  7.80 µs  |  3.11 µs  |
| threaded_scheduler_spawn_10   | 22.94 µs  |  4.65 µs  |
| threaded_scheduler_spawn_100  | 23.82 µs  | 24.37 µs  |
| threaded_scheduler_spawn_1000 | 123.41 µs | 274.15 µs |


## sync_broadcast

| Test Name       |   Nio    |  Tokio   |
| --------------- | :------: | :------: |
| contention/10   | 1.95 ms  | 1.83 ms  |
| contention/100  | 4.72 ms  | 4.98 ms  |
| contention/500  | 17.45 ms | 16.25 ms |
| contention/1000 | 34.45 ms | 30.29 ms |

## sync_mpsc_oneshot

| Test Name        |    Nio    |  Tokio   |
| ---------------- | :-------: | :------: |
| request_reply    | 222.54 µs | 1.979 ms |
| request_reply #2 | 233.26 µs | 3.180 ms |

## sync_mpsc

| Test Name             |    Nio    |   Tokio   |
| --------------------- | :-------: | :-------: |
| contention/unbounded  | 422.93 µs | 430.35 µs |
| uncontented/unbounded | 202.55 µs | 180.97 µs |

## sync_notify

| Test Name          |    Nio    |   Tokio   |
| ------------------ | :-------: | :-------: |
| notify_one/10      | 152.90 µs | 90.60 µs  |
| notify_one/50      | 235.92 µs | 88.26 µs  |
| notify_one/100     | 240.42 µs | 84.52 µs  |
| notify_one/200     | 249.60 µs | 88.56 µs  |
| notify_one/500     | 251.92 µs | 87.61 µs  |
| notify_waiters/10  | 158.44 µs | 119.18 µs |
| notify_waiters/50  | 247.32 µs | 122.08 µs |
| notify_waiters/100 | 278.90 µs | 122.19 µs |
| notify_waiters/200 | 331.49 µs | 123.10 µs |
| notify_waiters/500 | 424.12 µs | 123.60 µs |

### sync_semaphore