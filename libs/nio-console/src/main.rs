use std::{thread, time::Duration};

#[nio::main]
async fn main() {
    nio_console::launch();
    loop {
        nio::sleep(Duration::from_secs(2)).await;
        thread::sleep(Duration::from_secs(1));
    }
}
