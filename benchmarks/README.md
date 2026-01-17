## Run Benchmark

```sh
cd benchmarks
cargo bench -F tokio --bench rt_multi_threaded
cargo bench --bench rt_multi_threaded
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