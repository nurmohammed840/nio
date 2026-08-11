mod app;
mod style;

use nio::RuntimeContext;

use crate::app::App;
use std::io::Result;

#[allow(warnings)]
pub fn launch() {
    let last_worker = RuntimeContext::current().metrics().num_workers() - 1;
    nio::spawn_pinned_at(last_worker.try_into().unwrap(), launch_init);
}

async fn launch_init() {
    if let Err(err) = init().await {
        eprintln!("{err:#?}");
    }
}

pub async fn init() -> Result<()> {
    let terminal = ratatui::init();
    let result = App::new().run(terminal).await;
    ratatui::restore();
    result
}
