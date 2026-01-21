# Nio Task

Nio Task is a low-level task abstraction designed for building executors. It gives you explicit control over how and when a task is polled and rescheduled.

[![Crates.io](https://img.shields.io/crates/v/nio-task.svg)](https://crates.io/crates/nio-task)
[![Documentation](https://docs.rs/nio-task/badge.svg)](https://docs.rs/nio-task)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/license/apache-2-0)

## Example

Add this to your `Cargo.toml` file:

```toml
[dependencies]
nio-task = "0.1"
```

```rust
use nio_task::{Status, Task};

let metadata = 0;
let future = async {};
let scheduler = |task| {
    // ...
};

let (task, join) = Task::new_with(metadata, future, scheduler);

match task.poll() {
    Status::Yielded(task) => task.schedule(),
    Status::Pending => {}
    Status::Complete(_metadata) => {}
}
```