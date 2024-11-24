use crate::runtime::task::BlockingTask;
use mpmc_channel::MPMC;
use std::{collections::VecDeque, sync::Arc, thread, time::Duration};

type Channel = Arc<MPMC<VecDeque<BlockingTask>>>;

#[derive(Clone, Debug)]
pub struct ThreadPoolConfig {
    pub max_workers: u16,
    pub timeout: Option<Duration>,
    pub stack_size: Option<usize>,
    pub name: Option<String>,
}

#[derive(Default, Debug)]
pub struct ThreadPool {
    channel: Channel,
    config: ThreadPoolConfig,
}

pub trait SpawnBlocking: Send + Sync {
    fn spawn_blocking(&self, _: BlockingTask);
}

impl SpawnBlocking for ThreadPool {
    fn spawn_blocking(&self, task: BlockingTask) {
        let mut tx = self.channel.produce();
        tx.push_back(task);
        tx.notify_one();

        let num_of_workers = Arc::strong_count(&self.channel);
        if num_of_workers > self.config.max_workers.into() {
            return;
        }

        let timeout = self.config.timeout;
        let channel = self.channel.clone();

        let worker = move || {
            let mut rx = channel.consume();
            loop {
                rx = match rx.pop_front() {
                    Some(task) => {
                        drop(rx);
                        task.run();
                        channel.consume()
                    }
                    None => match timeout {
                        None => rx.wait(),
                        Some(dur) => match rx.wait_timeout(dur) {
                            Ok(rx) => rx,
                            Err(_) => break,
                        },
                    },
                }
            }
        };

        self.config
            .thread()
            .spawn(worker)
            .expect("failed to spawn worker thread");
    }
}

impl ThreadPoolConfig {
    fn thread(&self) -> thread::Builder {
        let mut thread = thread::Builder::new();
        if let Some(size) = self.stack_size {
            thread = thread.stack_size(size);
        }
        if let Some(name) = &self.name {
            thread = thread.name(name.clone());
        }
        thread
    }
}

impl Default for ThreadPoolConfig {
    fn default() -> Self {
        Self {
            timeout: Some(Duration::from_secs(10)),
            max_workers: 512,
            stack_size: None,
            name: None,
        }
    }
}

impl From<ThreadPoolConfig> for ThreadPool {
    fn from(config: ThreadPoolConfig) -> Self {
        Self {
            channel: Default::default(),
            config,
        }
    }
}

impl From<ThreadPoolConfig> for Arc<dyn SpawnBlocking> {
    fn from(config: ThreadPoolConfig) -> Self {
        Arc::new(ThreadPool::from(config))
    }
}
