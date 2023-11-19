mod search;
mod settings;
mod lineview;
mod main_window;

use lazy_static::lazy_static;
use log::{error};
use std::sync::RwLock;
use crate::main_window::GorlMainWindow;


lazy_static! {
    static ref SETTINGS: RwLock<settings::Settings> = RwLock::new(settings::Settings::new());
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .max_blocking_threads(8192) // basically the limit of log file one can open
        .build()
        .unwrap();

        let fst = rt.spawn_blocking(|| {
            let (tx, rx) = flume::unbounded();
            let my = GorlMainWindow::new(rx.clone(), tx.clone()); // instantiate our main window
            if let Err(e) = my.wnd.run_main(None) {
                // ... and run it
                error!("{}", e);
            }
        });

    rt.block_on(async move {
       let _ = fst.await;
    });

    Ok(())
}
