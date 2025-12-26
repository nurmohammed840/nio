## Example

```rust
use nio_task::{Status, Task};

let future = async {};

let metadata = 0;
let scheduler = |_task| {
    // ...
};
let (task, _join) = Task::new_with(metadata, future, scheduler);

match task.poll() {
    Status::Yielded(task) => task.schedule(),
    Status::Pending => {}
    Status::Complete(_metadata) => {}
}
```