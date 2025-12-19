## Example

```rust
use nio_threadpool::{Runnable, ThreadPool};
use std::{thread, time::Duration};

struct Task {}

impl Runnable for Task {
    fn run(self) {
        println!("{:#?}", thread::current());
    }
}

#[test]
fn example() {
    let pool = ThreadPool::new()
        .max_threads_limit(2)
        .timeout(Some(Duration::from_secs(3)))
        .stack_size(512)
        .name(|id| format!("Worker-{id}"));

    pool.execute(Task {});
    pool.execute(Task {});
    pool.execute(Task {});
}
```