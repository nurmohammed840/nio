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

| Test Name         |  Nio   | Tokio  |
| ----------------- | :----: | :----: |
| spawn_many_local  | 105 µs | 166 µs |
| spawn_many_remote | 96 µs  | 165 µs |


## rt_multi_threaded

| Test Name               |  Nio   | Tokio |
| ----------------------- | :----: | :---: |
| spawn_many_local        | 2.5 ms |   —   |
| spawn_many_remote       | 1.1 ms |   —   |
| spawn_many_remote_idle  | 1.7 ms |   —   |
| spawn_many_remote_busy1 |  1 ms  |   —   |
| spawn_many_remote_busy2 | 126 µs |   —   |
| ping_pong               | 347 µs |   —   |
| yield_many              | 2.9 ms |   —   |
| chained_spawn           | 107 µs |   —   |

## spawn_blocking

| Test Name                     |   Nio    |  Tokio   |
| ----------------------------- | :------: | :------: |
| spawn_blocking/concurrency/1  | 1.99 ms  | 2.89 ms  |
| spawn_blocking/concurrency/2  | 4.13 ms  | 3.99 ms  |
| spawn_blocking/concurrency/4  | 9.03 ms  | 10.61 ms |
| spawn_blocking/concurrency/8  | 16.19 ms | 23.66 ms |
| spawn_blocking/concurrency/16 | 25.87 ms | 50.29 ms |

## spawn

| Test Name                     |    Nio    |   Tokio   |
| ----------------------------- | :-------: | :-------: |
| basic_scheduler_spawn         |  7.68 µs  |  2.26 µs  |
| basic_scheduler_spawn_10      | 12.39 µs  |  3.83 µs  |
| basic_scheduler_spawn_100     | 23.39 µs  | 24.34 µs  |
| basic_scheduler_spawn_1000    | 140.52 µs | 219.70 µs |
| threaded_scheduler_spawn      |  7.88 µs  |  3.12 µs  |
| threaded_scheduler_spawn_10   | 23.17 µs  |  4.19 µs  |
| threaded_scheduler_spawn_100  | 24.43 µs  | 22.89 µs  |
| threaded_scheduler_spawn_1000 | 124.88 µs | 257.35 µs |

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